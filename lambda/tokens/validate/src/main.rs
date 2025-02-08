pub mod requests;

use crate::requests::{TokenValidateRequest, TokenValidateResponse};

use shared::aws::cognito::token_authorizer::CognitoTokenAuthorizer;
use shared::aws::dynamodb::client::DynamoDbClient;
use shared::aws::lambda_events::{request::LambdaEventRequestHandler, response::apigw_response};
use shared::entity::secrets::Secrets;
use shared::repository::user_repository::{UserRepository, UserRepositoryImpl};
use shared::utils::env::get_env;

use anyhow::{anyhow, Error as AnyhowError};
use aws_lambda_events::event::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};
use aws_lambda_events::http::{HeaderMap, HeaderValue};
use lambda_runtime::{service_fn, Error, LambdaEvent};
use tracing::{debug, error, info, instrument};

#[instrument(name = "lambda.tokens.validate.initialize_user_repository")]
async fn initialize_user_repository(
    region_string: String,
) -> Result<UserRepositoryImpl, AnyhowError> {
    let client = DynamoDbClient::new(region_string.clone()).await?;
    let table_name = get_env("TABLE_NAME", "Users");
    Ok(UserRepositoryImpl::new(client, table_name))
}

#[instrument(name = "lambda.tokens.refresh.initialize_cognito_token_authorizer_client")]
async fn initialize_cognito_token_authorizer_client(
    region_string: String,
) -> Result<CognitoTokenAuthorizer, Error> {
    let secrets = Secrets::get_secrets(region_string.clone()).await?;
    let authorizer = CognitoTokenAuthorizer::new(
        secrets.user_pool_id,
        secrets.jwks_url,
        region_string.clone(),
    )
    .await;
    Ok(authorizer)
}

#[instrument(name = "lambda.tokens.refresh.parse_validate_token_request")]
fn parse_validate_token_request(body: Option<&str>) -> Result<TokenValidateRequest, AnyhowError> {
    let body_str = body.ok_or_else(|| anyhow!("Missing request body"))?;
    let request: TokenValidateRequest = serde_json::from_str(body_str)
        .map_err(|e| anyhow!("Failed to parse TokenValidateRequest: {}", e))?;
    Ok(request)
}

#[instrument(name = "lambda.tokens.validate.token_validate_handler")]
async fn token_validate_handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    let region_string = get_env("REGION", "ap-northeast-1");
    let client = initialize_cognito_token_authorizer_client(region_string.clone()).await?;
    let repository = initialize_user_repository(region_string).await?;
    let validate_token_request = parse_validate_token_request(event.payload.body.as_deref())?;

    let claims = client.validate_token(&validate_token_request.token).await;
    match claims {
        Ok(claims) => {
            let user = repository.get_user_by_id(claims.sub.to_string()).await?;
            let response = TokenValidateResponse {
                user_id: user.id,
                organization_id: user.organization_id,
            };

            // Set user_id and organization_id to lambda context
            let mut headers = HeaderMap::new();
            headers.insert("user_id", HeaderValue::from_str(&response.user_id)?);
            headers.insert(
                "organization_id",
                HeaderValue::from_str(&response.organization_id)?,
            );

            Ok(apigw_response(
                200,
                Some(serde_json::to_string(&response)?.into()),
                Some(headers),
            ))
        }
        Err(e) => {
            error!("Validate token failed: {:?}", e);
            Ok(apigw_response(403, Some("Invalid token".into()), None))
        }
    }
}

#[instrument(name = "lambda.tokens.validate.handler")]
async fn handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    debug!("handling lambda req: {:?}", event);
    LambdaEventRequestHandler::handle_requests(event, "/tokens/validate", token_validate_handler)
        .await
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    shared::tracer::init_tracing();
    info!("Starting auth token validate function");
    lambda_runtime::run(service_fn(handler)).await
}
