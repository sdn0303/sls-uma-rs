mod requests;

use crate::requests::{SignupRequest, SignupResponse};

use shared::aws::lambda_events::{request::LambdaEventRequestHandler, response::apigw_response};
use shared::client_manager::{CognitoClientManager, DefaultClientManager, DynamoDbClientManager};
use shared::entity::user::{Role, User};
use shared::errors::{LambdaError, LambdaResult, ToLambdaError};
use shared::repository::user_repository::{UserRepository, UserRepositoryImpl};
use shared::utils::{env::get_env, uuid::generate_uuid};

use aws_lambda_events::event::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};
use lambda_runtime::{service_fn, Error, LambdaEvent};
use std::collections::HashSet;
use tracing::{debug, info, instrument};

/// Generate new user with appropriate role based on organization existence
async fn generate_new_user(
    id: String,
    request: SignupRequest,
    repository: &impl UserRepository,
) -> LambdaResult<User> {
    let mut roles = HashSet::new();

    // Check if organization exists
    let organization_id = match repository
        .find_organization_id_by_name(&request.organization_name)
        .await
        .map_err(|e| LambdaError::InternalError(e.to_string()))?
    {
        Some(existing_org_id) => {
            info!("Found existing organization: {}", existing_org_id);
            roles.insert(Role::Writer);
            existing_org_id
        }
        None => {
            info!(
                "Creating new organization for: {}",
                request.organization_name
            );
            roles.insert(Role::Admin);
            generate_uuid()
        }
    };

    Ok(User::new(
        id,
        request.user_name,
        request.email,
        organization_id,
        request.organization_name,
        roles,
    ))
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

#[instrument(name = "lambda.auth.signup.signup_handler")]
async fn signup_handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    let client_manager = DefaultClientManager::new("ap-northeast-1".to_string());

    // Zero-copy deserialization and validation
    let body = event
        .payload
        .body
        .as_deref()
        .ok_or_else(|| Error::from(LambdaError::MissingBody))?;

    let signup_request: SignupRequest =
        serde_json::from_slice(body.as_bytes()).map_err(|e| Error::from(e.to_lambda_error()))?;

    // Validation
    if let Err(e) = signup_request.validate() {
        return create_error_response(e);
    }

    // Get clients using abstraction with explicit trait disambiguation
    let cognito_client = CognitoClientManager::get_client(&client_manager)
        .await
        .map_err(|e| Error::from(e))?;
    let dynamodb_client = DynamoDbClientManager::get_client(&client_manager)
        .await
        .map_err(|e| Error::from(e))?;

    let table_name = get_env("TABLE_NAME", "Users");
    let repository = UserRepositoryImpl::new((*dynamodb_client).clone(), table_name);

    // Try to create user in Cognito
    match cognito_client
        .admin_create_user(signup_request.email.clone())
        .await
    {
        Ok(admin_create_user_opt) => {
            debug!("admin create user output: {:?}", admin_create_user_opt);

            let opt = cognito_client
                .admin_set_user_password(
                    &signup_request.email.clone(),
                    &signup_request.password.clone(),
                    true,
                )
                .await
                .map_err(|e| Error::from(LambdaError::InternalError(e.to_string())))?;
            debug!("admin set user password output: {:?}", opt);

            let opt = cognito_client
                .email_verified(signup_request.email.clone(), signup_request.email.clone())
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

            let new_user = generate_new_user(sub.to_string(), signup_request, &repository)
                .await
                .map_err(|e| Error::from(e))?;

            repository
                .create_user(new_user)
                .await
                .map_err(|e| Error::from(LambdaError::UserCreationFailed(e.to_string())))?;

            let response = SignupResponse {
                message: "signup successfully.".to_string(),
            };
            Ok(apigw_response(
                200,
                Some(serde_json::to_string(&response)?.into()),
                None,
            ))
        }
        Err(e) => {
            let error = if e.to_string().contains("UsernameExistsException") {
                LambdaError::UserAlreadyExists
            } else if e.to_string().contains("InvalidPasswordException") {
                LambdaError::InvalidPassword
            } else {
                debug!("Signup error: {:?}", e);
                LambdaError::InternalError(e.to_string())
            };
            create_error_response(error)
        }
    }
}

#[instrument(name = "lambda.auth.signup.handler")]
async fn handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    debug!("handling lambda req: {:?}", event);
    LambdaEventRequestHandler::handle_requests(event, "/signup", signup_handler).await
}

// Custom allocator configuration
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() -> Result<(), Error> {
    shared::tracer::init_tracing();
    info!("Starting auth user signup function");
    lambda_runtime::run(service_fn(handler)).await
}
