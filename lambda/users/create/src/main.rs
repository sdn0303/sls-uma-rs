mod requests;

use crate::requests::{CreateUserRequest, CreateUserResponse};

use shared::aws::lambda_events::{request::LambdaEventRequestHandler, response::apigw_response};
use shared::cache_manager::get_cache_manager;
use shared::client_manager::{CognitoClientManager, DefaultClientManager, DynamoDbClientManager};
use shared::entity::user::{Permissions, Role, User};
use shared::errors::{LambdaError, LambdaResult, ToLambdaError};
use shared::repository::user_repository::{UserRepository, UserRepositoryImpl};
use shared::utils::{env::get_env, password::generate_password};

use aws_lambda_events::event::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};
use lambda_runtime::{service_fn, Error, LambdaEvent};
use std::collections::HashSet;
use tracing::{debug, error, info, instrument};

/// Check create permission with caching
async fn check_create_permission_with_cache(user: &User, user_id: &str) -> LambdaResult<()> {
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
    let has_permission = user.has_permission(Permissions::CREATE);
    cache_manager
        .set_permission(user_id.to_string(), has_permission)
        .await;

    if has_permission {
        Ok(())
    } else {
        Err(LambdaError::InsufficientPermissions)
    }
}

/// Generate new user
fn generate_new_user(id: String, request: CreateUserRequest) -> LambdaResult<User> {
    let roles = HashSet::new();
    let mut user = User::new(
        id,
        request.user_name,
        request.email,
        request.organization_id,
        request.organization_name,
        roles,
    );
    user.set_from_roles(request.roles.clone());
    Ok(user)
}

/// Build create user response
fn build_create_user_response(
    user: &User,
    tmp_password: String,
) -> LambdaResult<CreateUserResponse> {
    let roles = user.roles.iter().cloned().collect::<Vec<Role>>();
    Ok(CreateUserResponse {
        user_name: user.name.clone(),
        user_email: user.email.clone(),
        user_roles: roles,
        user_tmp_password: tmp_password,
    })
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

#[instrument(name = "lambda.users.create.create_user_handler")]
async fn create_user_handler(
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

    let create_request: CreateUserRequest =
        serde_json::from_slice(body.as_bytes()).map_err(|e| Error::from(e.to_lambda_error()))?;

    // Validation
    if let Err(e) = create_request.validate() {
        return create_error_response(e);
    }

    // Get clients using abstraction with explicit trait disambiguation
    let dynamodb_client = DynamoDbClientManager::get_client(&client_manager)
        .await
        .map_err(Error::from)?;
    let cognito_client = CognitoClientManager::get_client(&client_manager)
        .await
        .map_err(Error::from)?;

    let table_name = get_env("TABLE_NAME", "Users");
    let repository = UserRepositoryImpl::new((*dynamodb_client).clone(), table_name);

    // Permission check
    let user = repository
        .get_user_by_id(user_id.clone())
        .await
        .map_err(|e| Error::from(LambdaError::UserRetrievalFailed(e.to_string())))?;

    if let Err(e) = check_create_permission_with_cache(&user, &user_id).await {
        return create_error_response(e);
    }

    let tmp_password =
        generate_password().map_err(|e| Error::from(LambdaError::InternalError(e.to_string())))?;
    debug!("Password has been generated");

    // Try to create user in Cognito
    match cognito_client
        .admin_create_user(create_request.email.clone())
        .await
    {
        Ok(admin_create_user_opt) => {
            debug!("admin create user output: {:?}", admin_create_user_opt);

            let opt = cognito_client
                .admin_set_user_password(&create_request.email.clone(), &tmp_password, true)
                .await
                .map_err(|e| Error::from(LambdaError::InternalError(e.to_string())))?;
            debug!("admin set user password output: {:?}", opt);

            let opt = cognito_client
                .email_verified(create_request.email.clone(), create_request.email.clone())
                .await
                .map_err(|e| Error::from(LambdaError::InternalError(e.to_string())))?;
            debug!("email verified user output: {:?}", opt);

            let sub = admin_create_user_opt
                .user()
                .ok_or_else(|| Error::from(LambdaError::InternalError("user is None".to_string())))?
                .attributes()
                .iter()
                .find(|attr| attr.name() == "sub")
                .ok_or_else(|| Error::from(LambdaError::InternalError("sub is None".to_string())))?
                .value()
                .ok_or_else(|| {
                    Error::from(LambdaError::InternalError("sub value is None".to_string()))
                })?;

            let new_user =
                generate_new_user(sub.to_string(), create_request).map_err(Error::from)?;
            let created_user = repository
                .create_user(new_user)
                .await
                .map_err(|e| Error::from(LambdaError::UserCreationFailed(e.to_string())))?;
            let response =
                build_create_user_response(&created_user, tmp_password).map_err(Error::from)?;

            Ok(apigw_response(
                200,
                Some(serde_json::to_string(&response)?.into()),
                None,
            ))
        }
        Err(e) => {
            let error = if e.to_string().contains("UsernameExistsException") {
                LambdaError::UserAlreadyExists
            } else {
                error!("Failed to create user in Cognito: {:?}", e);
                LambdaError::UserCreationFailed(e.to_string())
            };
            create_error_response(error)
        }
    }
}

#[instrument(name = "lambda.users.create.handler")]
async fn handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    debug!("handling lambda req: {:?}", event);
    LambdaEventRequestHandler::handle_requests(
        event,
        "/organizations/{organizationId}/users",
        create_user_handler,
    )
    .await
}

// Custom allocator configuration
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() -> Result<(), Error> {
    shared::tracer::init_tracing();
    info!("Starting auth user create function");
    lambda_runtime::run(service_fn(handler)).await
}
