pub mod requests;

use crate::requests::{TokenValidateRequest, TokenValidateResponse};

use shared::aws::lambda_events::{request::LambdaEventRequestHandler, response::apigw_response};
use shared::cache_manager::get_cache_manager;
use shared::client_manager::{DefaultClientManager, DynamoDbClientManager, TokenAuthorizerManager};
use shared::entity::user::User;
use shared::errors::{LambdaError, LambdaResult, ToLambdaError};
use shared::repository::user_repository::{UserRepository, UserRepositoryImpl};
use shared::utils::env::get_env;

use aws_lambda_events::event::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};
use aws_lambda_events::http::{HeaderMap, HeaderValue};
use lambda_runtime::{service_fn, Error, LambdaEvent};
use tracing::{debug, error, info, instrument};

/// Get user info with caching
async fn get_user_with_cache(
    user_id: &str,
    client_manager: &DefaultClientManager,
) -> LambdaResult<User> {
    let cache_manager = get_cache_manager();

    // Check cache first
    if let Some(cached_user) = cache_manager.get_user(user_id).await {
        debug!("User info cache hit for user: {}", user_id);
        return Ok(cached_user);
    }

    // Get user from database on cache miss
    let dynamodb_client = client_manager.get_client().await?;
    let table_name = get_env("TABLE_NAME", "Users");
    let repository = UserRepositoryImpl::new((*dynamodb_client).clone(), table_name);

    let user = repository
        .get_user_by_id(user_id.to_string())
        .await
        .map_err(|e| LambdaError::UserRetrievalFailed(e.to_string()))?;

    cache_manager
        .set_user(user_id.to_string(), user.clone())
        .await;
    Ok(user)
}

/// Create standardized error response
fn create_error_response(error: LambdaError) -> Result<ApiGatewayProxyResponse, Error> {
    let error_response = serde_json::json!({
        "error": error.to_string(),
        "message": error.user_message()
    });

    Ok(apigw_response(
        error.status_code(),
        Some(serde_json::to_string(&error_response)?.into()),
        None,
    ))
}

#[instrument(name = "lambda.tokens.validate.token_validate_handler")]
async fn token_validate_handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    let client_manager = DefaultClientManager::new("ap-northeast-1".to_string());

    // Zero-copy deserialization and validation
    let body = event
        .payload
        .body
        .as_deref()
        .ok_or_else(|| Error::from(LambdaError::MissingBody))?;

    let validate_request: TokenValidateRequest =
        serde_json::from_slice(body.as_bytes()).map_err(|e| Error::from(e.to_lambda_error()))?;

    // Validation
    if let Err(e) = validate_request.validate() {
        return create_error_response(e);
    }

    // Get token authorizer using abstraction
    let authorizer = client_manager.get_authorizer().await.map_err(Error::from)?;

    let claims = match authorizer.validate_token(&validate_request.token).await {
        Ok(claims) => claims,
        Err(e) => {
            let error = if e.to_string().contains("expired") {
                LambdaError::TokenExpired
            } else if e.to_string().contains("signature") {
                LambdaError::InvalidSignature
            } else {
                error!("Token validation error: {:?}", e);
                LambdaError::InternalError(e.to_string())
            };
            return create_error_response(error);
        }
    };

    // Get user info with caching
    let user = get_user_with_cache(&claims.sub, &client_manager)
        .await
        .map_err(Error::from)?;

    let response = TokenValidateResponse {
        user_id: user.id.clone(),
        organization_id: user.organization_id.clone(),
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

#[instrument(name = "lambda.tokens.validate.handler")]
async fn handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    debug!("handling lambda req: {:?}", event);
    LambdaEventRequestHandler::handle_requests(event, "/tokens/validate", token_validate_handler)
        .await
}

// Custom allocator configuration
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() -> Result<(), Error> {
    shared::tracer::init_tracing();
    info!("Starting auth token validate function");
    lambda_runtime::run(service_fn(handler)).await
}
