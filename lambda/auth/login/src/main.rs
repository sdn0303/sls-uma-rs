mod requests;

use crate::requests::{LoginRequest, LoginResponse};

use shared::aws::cognito::client::CognitoClient;
use shared::aws::lambda_events::{request::LambdaEventRequestHandler, response::apigw_response};
use shared::entity::secrets::Secrets;
use shared::utils::env::get_env;

use aws_lambda_events::event::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};
use lambda_runtime::{service_fn, Error, LambdaEvent};
use tracing::{debug, info, instrument};

#[instrument(name = "lambda.auth.login.initialize_cognito_client")]
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

#[instrument(name = "lambda.auth.login.login_handler")]
async fn login_handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    let (user_id, _) =
        LambdaEventRequestHandler::get_ids_from_request_context(event.clone()).await?;
    let client = initialize_cognito_client().await?;
    let login_request: LoginRequest = serde_json::from_str(event.payload.body.as_deref().unwrap())?;

    let hash = client.calculate_hash(user_id.clone()).await?;
    let opt = client
        .user_login(user_id, login_request.email, login_request.password, hash)
        .await?;

    match opt.authentication_result() {
        Some(result) => {
            let response = LoginResponse {
                access_token: result
                    .access_token
                    .as_deref()
                    .unwrap_or("Missing access_token")
                    .to_string(),
                id_token: result
                    .id_token
                    .as_deref()
                    .unwrap_or("Missing id_token")
                    .to_string(),
                refresh_token: result
                    .refresh_token
                    .as_deref()
                    .unwrap_or("Missing refresh_token")
                    .to_string(),
            };
            Ok(apigw_response(
                200,
                Some(serde_json::to_string(&response)?.into()),
                None,
            ))
        }
        None => Ok(apigw_response(500, Some("Internal Error".into()), None)),
    }
}

#[instrument(name = "lambda.auth.login.handler")]
async fn handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    debug!("handling lambda req: {:?}", event);
    LambdaEventRequestHandler::handle_requests(event, "/login", login_handler).await
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    shared::tracer::init_tracing();
    info!("Starting auth user login function");
    lambda_runtime::run(service_fn(handler)).await
}
