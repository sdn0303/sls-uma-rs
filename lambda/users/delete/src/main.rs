mod requests;

use crate::requests::DeleteUserResponse;

use shared::aws::lambda_events::{request::LambdaEventRequestHandler, response::apigw_response};
use shared::cache_manager::get_cache_manager;
use shared::client_manager::{CognitoClientManager, DefaultClientManager, DynamoDbClientManager};
use shared::entity::user::{Permissions, User};
use shared::errors::{LambdaError, LambdaResult};
use shared::repository::user_repository::{UserRepository, UserRepositoryImpl};
use shared::utils::env::get_env;

use aws_lambda_events::event::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};
use lambda_runtime::{service_fn, Error, LambdaEvent};
use tracing::{debug, info, instrument};

/// Check delete permission with caching
async fn check_delete_permission_with_cache(user: &User, user_id: &str) -> LambdaResult<()> {
    let cache_manager = get_cache_manager();

    // Check cache first
    if let Some(has_permission) = cache_manager.get_permission(user_id).await {
        debug!("Permission cache hit for user: {}", user_id);
        return if has_permission {
            Ok(())
        } else {
            Err(LambdaError::InsufficientPermissions)
        };
    }

    // Check permission on cache miss
    let has_permission = user.has_permission(Permissions::DELETE);
    cache_manager
        .set_permission(user_id.to_string(), has_permission)
        .await;

    if has_permission {
        Ok(())
    } else {
        Err(LambdaError::InsufficientPermissions)
    }
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

#[instrument(name = "lambda.users.delete.delete_user_handler")]
async fn delete_user_handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    let client_manager = DefaultClientManager::new("ap-northeast-1".to_string());

    let (user_id, organization_id) =
        LambdaEventRequestHandler::get_ids_from_request_context(event.clone()).await?;

    // Get clients using abstraction with explicit trait disambiguation
    let dynamodb_client = DynamoDbClientManager::get_client(&client_manager)
        .await
        .map_err(|e| Error::from(e))?;
    let cognito_client = CognitoClientManager::get_client(&client_manager)
        .await
        .map_err(|e| Error::from(e))?;

    let table_name = get_env("TABLE_NAME", "Users");
    let repository = UserRepositoryImpl::new((*dynamodb_client).clone(), table_name);

    // Permission check
    let user = repository
        .get_user_by_id(user_id.clone())
        .await
        .map_err(|e| Error::from(LambdaError::UserRetrievalFailed(e.to_string())))?;

    if let Err(e) = check_delete_permission_with_cache(&user, &user_id).await {
        return create_error_response(e);
    }

    // Delete user from Cognito
    cognito_client
        .admin_delete_user(user_id.clone())
        .await
        .map_err(|e| Error::from(LambdaError::UserDeletionFailed(e.to_string())))?;

    // Delete user from DynamoDB
    repository
        .delete_user_by_id(user_id.clone(), organization_id.clone())
        .await
        .map_err(|e| Error::from(LambdaError::UserDeletionFailed(e.to_string())))?;

    let response = DeleteUserResponse {
        message: format!("User {} has been deleted.", user_id),
    };
    Ok(apigw_response(
        200,
        Some(serde_json::to_string(&response)?.into()),
        None,
    ))
}

#[instrument(name = "lambda.users.delete.handler")]
async fn handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    debug!("handling lambda req: {:?}", event);
    LambdaEventRequestHandler::handle_requests(
        event,
        "/organizations/{organizationId}/users/{userId}",
        delete_user_handler,
    )
    .await
}

// Custom allocator configuration
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() -> Result<(), Error> {
    shared::tracer::init_tracing();
    info!("Starting auth user delete function");
    lambda_runtime::run(service_fn(handler)).await
}
