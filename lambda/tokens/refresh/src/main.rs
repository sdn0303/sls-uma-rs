mod requests;

use crate::requests::{RefreshTokenRequest, RefreshTokenResponse};

use shared::aws::cognito::client::CognitoClient;
use shared::aws::lambda_events::{request::LambdaEventRequestHandler, response::apigw_response};
use shared::entity::secrets::Secrets;
use shared::utils::env::get_env;

use anyhow::{anyhow, Error as AnyhowError};
use aws_lambda_events::event::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};
use lambda_runtime::{service_fn, Error, LambdaEvent};
use tracing::{debug, error, info, instrument};

#[instrument(name = "lambda.tokens.refresh.initialize_cognito_client")]
async fn initialize_cognito_client() -> Result<CognitoClient, Error> {
    let region_string = get_env("REGION", "ap-northeast-1");
    let secrets = Secrets::get_secrets(region_string.clone()).await?;
    let client = CognitoClient::new(
        region_string,
        secrets.user_pool_id,
        secrets.client_id,
        secrets.client_secret,
    )
    .await?;
    Ok(client)
}

#[instrument(name = "lambda.tokens.refresh.parse_refresh_token_request")]
fn parse_refresh_token_request(body: Option<&str>) -> Result<RefreshTokenRequest, AnyhowError> {
    let body_str = body.ok_or_else(|| anyhow!("Missing request body"))?;
    let request: RefreshTokenRequest = serde_json::from_str(body_str)
        .map_err(|e| anyhow!("Failed to parse RefreshTokenRequest: {}", e))?;
    Ok(request)
}

#[instrument(name = "lambda.tokens.refresh.refresh_token_handler")]
async fn refresh_token_handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    let (user_id, _) =
        LambdaEventRequestHandler::get_ids_from_request_context(event.clone()).await?;
    let client = initialize_cognito_client().await?;
    let refresh_token_request = parse_refresh_token_request(event.payload.body.as_deref())?;
    if refresh_token_request.grant_type != "refresh_token" {
        return Ok(apigw_response(400, Some("Bad Request".into()), None));
    };
    let hash = client.calculate_hash(user_id.to_string()).await?;
    let result = client
        .refresh_token(refresh_token_request.refresh_token, hash)
        .await?;
    match result.authentication_result() {
        Some(res) => {
            let access_token = res
                .access_token
                .as_deref()
                .unwrap_or("Missing access_token")
                .to_string();
            let refresh_token = res
                .refresh_token
                .as_deref()
                .unwrap_or("Missing refresh_token")
                .to_string();
            let response = RefreshTokenResponse {
                access_token,
                refresh_token,
            };
            Ok(apigw_response(
                200,
                Some(serde_json::to_string(&response)?.into()),
                None,
            ))
        }
        None => {
            error!("Failed to refresh token");
            Ok(apigw_response(500, Some("Internal Error".into()), None))
        }
    }
}

#[instrument(name = "lambda.tokens.refresh.handler")]
async fn handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    debug!("handling lambda req: {:?}", event);
    LambdaEventRequestHandler::handle_requests(event, "/tokens/refresh", refresh_token_handler)
        .await
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    shared::tracer::init_tracing();
    info!("Starting auth token refresh function");
    lambda_runtime::run(service_fn(handler)).await
}
