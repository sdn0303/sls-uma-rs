mod requests;

use crate::requests::ListUsersResponse;

use shared::aws::lambda_events::{request::LambdaEventRequestHandler, response::apigw_response};
use shared::cache_manager::get_cache_manager;
use shared::client_manager::{DefaultClientManager, DynamoDbClientManager};
use shared::errors::LambdaError;
use shared::repository::user_repository::{UserRepository, UserRepositoryImpl};
use shared::utils::env::get_env;

use aws_lambda_events::event::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};
use lambda_runtime::{service_fn, Error, LambdaEvent};
use tracing::{debug, info, instrument};

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

#[instrument(name = "lambda.users.get.get_user_handler")]
async fn get_user_handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    let client_manager = DefaultClientManager::new("ap-northeast-1".to_string());
    let cache_manager = get_cache_manager();

    let (user_id, _) =
        LambdaEventRequestHandler::get_ids_from_request_context(event.clone()).await?;

    // Get user info from cache
    let user = if let Some(cached_user) = cache_manager.get_user(&user_id).await {
        debug!("User info cache hit for user: {}", user_id);
        cached_user
    } else {
        let dynamodb_client = DynamoDbClientManager::get_client(&client_manager)
            .await
            .map_err(|e| Error::from(e))?;
        let table_name = get_env("TABLE_NAME", "Users");
        let repository = UserRepositoryImpl::new((*dynamodb_client).clone(), table_name);

        match repository.get_user_by_id(user_id.clone()).await {
            Ok(user) => {
                cache_manager.set_user(user_id.clone(), user.clone()).await;
                user
            }
            Err(_) => {
                return create_error_response(LambdaError::UserNotFound);
            }
        }
    };

    Ok(apigw_response(
        200,
        Some(serde_json::to_string(&user)?.into()),
        None,
    ))
}

#[instrument(name = "lambda.users.get.get_users_handler")]
async fn get_users_handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    let client_manager = DefaultClientManager::new("ap-northeast-1".to_string());
    let cache_manager = get_cache_manager();

    let (_, organization_id) =
        LambdaEventRequestHandler::get_ids_from_request_context(event.clone()).await?;

    // Get organization users list from cache
    let users = if let Some(cached_users) = cache_manager.get_org_users(&organization_id).await {
        debug!("Organization users cache hit for org: {}", organization_id);
        cached_users
    } else {
        let dynamodb_client = DynamoDbClientManager::get_client(&client_manager)
            .await
            .map_err(|e| Error::from(e))?;
        let table_name = get_env("TABLE_NAME", "Users");
        let repository = UserRepositoryImpl::new((*dynamodb_client).clone(), table_name);

        match repository
            .get_users_by_organization_id(organization_id.clone())
            .await
        {
            Ok(users) => {
                cache_manager
                    .set_org_users(organization_id.clone(), users.clone())
                    .await;
                users
            }
            Err(_) => {
                return create_error_response(LambdaError::OrganizationNotFound);
            }
        }
    };

    let response = ListUsersResponse { users };
    Ok(apigw_response(
        200,
        Some(serde_json::to_string(&response)?.into()),
        None,
    ))
}

#[instrument(name = "lambda.users.get.handler")]
async fn handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    debug!("handling lambda req: {:?}", event);
    let resource = event.clone().payload.resource.unwrap_or_default();
    match resource.as_str() {
        "/organizations/{organizationId}/users/{userId}" => {
            LambdaEventRequestHandler::handle_requests(
                event,
                "/organizations/{organizationId}/users/{userId}",
                get_user_handler,
            )
            .await
        }
        "/organizations/{organizationId}/users" => {
            LambdaEventRequestHandler::handle_requests(
                event,
                "/organizations/{organizationId}/users",
                get_users_handler,
            )
            .await
        }
        _ => {
            info!("Path not handled: {}", resource);
            Ok(apigw_response(404, Some("Not Found".into()), None))
        }
    }
}

// Custom allocator configuration
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() -> Result<(), Error> {
    shared::tracer::init_tracing();
    info!("Starting auth user get function");
    lambda_runtime::run(service_fn(handler)).await
}
