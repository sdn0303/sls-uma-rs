mod requests;

use crate::requests::{RefreshTokenRequest, RefreshTokenResponse};

use shared::aws::lambda_events::{request::LambdaEventRequestHandler, response::apigw_response};
use shared::cache_manager::get_cache_manager;
use shared::client_manager::{CognitoClientManager, DefaultClientManager};
use shared::errors::{LambdaError, LambdaResult, ToLambdaError};

use aws_lambda_events::event::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};
use lambda_runtime::{service_fn, Error, LambdaEvent};
use tracing::{debug, error, info, instrument};

/// Calculate hash with improved caching
async fn calculate_hash_with_cache(
    client: &shared::aws::cognito::client::CognitoClient,
    username: &str,
) -> LambdaResult<String> {
    let cache_manager = get_cache_manager();

    // Check cache first
    if let Some(hash) = cache_manager.get_hash(username).await {
        debug!("Hash cache hit for user: {}", username);
        return Ok(hash);
    }

    // Calculate hash on cache miss
    let hash = client
        .calculate_hash(username.to_string())
        .await
        .map_err(|e| LambdaError::InternalError(e.to_string()))?;

    cache_manager
        .set_hash(username.to_string(), hash.clone())
        .await;
    Ok(hash)
}

/// Create standardized error response
fn create_error_response(error: LambdaError) -> Result<ApiGatewayProxyResponse, Error> {
    let error_response = serde_json::json!({
        "error": error.to_string(),
        "message": error.user_message()
    });

    Ok(apigw_response(
        error.status_code() as i64,
        Some(serde_json::to_string(&error_response)?.into()),
        None,
    ))
}

#[instrument(name = "lambda.tokens.refresh.refresh_token_handler")]
async fn refresh_token_handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    let client_manager = DefaultClientManager::new("ap-northeast-1".to_string());

    let (user_id, _) =
        LambdaEventRequestHandler::get_ids_from_request_context(event.clone()).await?;

    // Zero-copy deserialization and validation
    let body = event
        .payload
        .body
        .as_deref()
        .ok_or_else(|| Error::from(LambdaError::MissingBody))?;

    let refresh_request: RefreshTokenRequest =
        serde_json::from_slice(body.as_bytes()).map_err(|e| Error::from(e.to_lambda_error()))?;

    // Validation
    if let Err(e) = refresh_request.validate() {
        return create_error_response(e);
    }

    // Get client using abstraction
    let client = client_manager
        .get_client()
        .await
        .map_err(|e| Error::from(e))?;

    let hash = calculate_hash_with_cache(&client, &user_id)
        .await
        .map_err(|e| Error::from(e))?;

    match client
        .refresh_token(refresh_request.refresh_token, hash)
        .await
    {
        Ok(result) => match result.authentication_result() {
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
                error!("Authentication result is None");
                create_error_response(LambdaError::InternalError(
                    "Failed to refresh token".to_string(),
                ))
            }
        },
        Err(e) => {
            let error = if e.to_string().contains("NotAuthorizedException") {
                LambdaError::InvalidRefreshToken
            } else if e.to_string().contains("ExpiredToken") {
                LambdaError::TokenExpired
            } else {
                error!("Refresh token error: {:?}", e);
                LambdaError::InternalError(e.to_string())
            };
            create_error_response(error)
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

// Custom allocator configuration
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() -> Result<(), Error> {
    shared::tracer::init_tracing();
    info!("Starting auth token refresh function");
    lambda_runtime::run(service_fn(handler)).await
}
