mod requests;

use crate::requests::{LoginRequest, LoginResponse};

use shared::aws::lambda_events::{request::LambdaEventRequestHandler, response::apigw_response};
use shared::cache_manager::get_cache_manager;
use shared::client_manager::{CognitoClientManager, DefaultClientManager, DynamoDbClientManager};
use shared::errors::{LambdaError, LambdaResult, ToLambdaError};
use shared::repository::user_repository::{UserRepository, UserRepositoryImpl};
use shared::utils::env::get_env;

use aws_lambda_events::event::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use lambda_runtime::{service_fn, Error, LambdaEvent};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    #[serde(flatten)]
    other: serde_json::Value,
}

/// Extract user ID from JWT token
fn extract_user_id_from_token(token: &str) -> Result<String, Box<dyn std::error::Error>> {
    // For ID tokens from Cognito, we can decode without verification for sub extraction
    // In production, you might want to verify the signature
    let mut validation = Validation::new(Algorithm::RS256);
    validation.insecure_disable_signature_validation();
    validation.validate_exp = false;
    validation.validate_aud = false;

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(&[]), // Empty key since we're not verifying
        &validation,
    )?;

    Ok(token_data.claims.sub)
}

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
        error.status_code(),
        Some(serde_json::to_string(&error_response)?.into()),
        None,
    ))
}

#[instrument(name = "lambda.auth.login.login_handler")]
async fn login_handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    let client_manager = DefaultClientManager::new("ap-northeast-1".to_string());

    // Zero-copy deserialization and validation
    let body = event
        .payload
        .body
        .as_deref()
        .ok_or_else(|| Error::from(LambdaError::MissingBody))?;

    let login_request: LoginRequest =
        serde_json::from_slice(body.as_bytes()).map_err(|e| Error::from(e.to_lambda_error()))?;

    // Validation
    if let Err(e) = login_request.validate() {
        return create_error_response(e);
    }

    // Get clients using abstraction with explicit trait disambiguation
    let cognito_client = CognitoClientManager::get_client(&client_manager)
        .await
        .map_err(Error::from)?;
    let dynamodb_client = DynamoDbClientManager::get_client(&client_manager)
        .await
        .map_err(Error::from)?;

    // Use email as username for Cognito authentication
    let username = login_request.email.clone();
    let hash = calculate_hash_with_cache(&cognito_client, &username)
        .await
        .map_err(Error::from)?;

    // Setup user repository
    let table_name = get_env("TABLE_NAME", "Users");
    let user_repository = UserRepositoryImpl::new((*dynamodb_client).clone(), table_name);

    match cognito_client
        .user_login(username, login_request.email, login_request.password, hash)
        .await
    {
        Ok(opt) => match opt.authentication_result() {
            Some(result) => {
                // Extract user_id from ID token (sub claim)
                let id_token = result.id_token.as_deref().ok_or_else(|| {
                    Error::from(LambdaError::InternalError("Missing id_token".to_string()))
                })?;

                // Parse JWT to get sub (user_id)
                let user_id = extract_user_id_from_token(id_token)
                    .map_err(|e| Error::from(LambdaError::InternalError(e.to_string())))?;

                // Get user information from DynamoDB
                let user = user_repository
                    .get_user_by_id(user_id.clone())
                    .await
                    .map_err(|_e| Error::from(LambdaError::UserNotFound))?;

                let response = LoginResponse {
                    access_token: result
                        .access_token
                        .as_deref()
                        .unwrap_or("Missing access_token")
                        .to_string(),
                    id_token: id_token.to_string(),
                    refresh_token: result
                        .refresh_token
                        .as_deref()
                        .unwrap_or("Missing refresh_token")
                        .to_string(),
                    user_id: user.id,
                    organization_id: user.organization_id,
                };
                Ok(apigw_response(
                    200,
                    Some(serde_json::to_string(&response)?.into()),
                    None,
                ))
            }
            None => {
                debug!("Authentication result is None");
                create_error_response(LambdaError::InternalError(
                    "Failed to authenticate".to_string(),
                ))
            }
        },
        Err(e) => {
            let error = if e.to_string().contains("NotAuthorizedException") {
                LambdaError::AuthenticationFailed
            } else if e.to_string().contains("UserNotFoundException") {
                LambdaError::UserNotFound
            } else {
                debug!("Login error: {:?}", e);
                LambdaError::InternalError(e.to_string())
            };
            create_error_response(error)
        }
    }
}

#[instrument(name = "lambda.auth.login.handler")]
async fn handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    debug!("handling lambda req: {:?}", event);
    LambdaEventRequestHandler::handle_requests(event, "/login", login_handler).await
}

// Custom allocator configuration
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() -> Result<(), Error> {
    shared::tracer::init_tracing();
    info!("Starting auth user login function");
    lambda_runtime::run(service_fn(handler)).await
}
