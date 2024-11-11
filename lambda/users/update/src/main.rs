mod requests;

use crate::requests::{UpdateUserRequest, UpdateUserResponse};
use shared::aws::dynamodb::client::DynamoDbClient;
use shared::aws::lambda_events::{request::LambdaEventRequestHandler, response::apigw_response};
use shared::repository::user_repository::{UserRepository, UserRepositoryImpl};
use shared::utils::env::get_env;

use anyhow::{anyhow, Error as AnyhowError};
use aws_lambda_events::event::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};
use lambda_runtime::{service_fn, Error, LambdaEvent};
use shared::entity::user::{Permissions, User};
use tracing::{debug, error, info, instrument};

#[instrument(name = "lambda.users.update.initialize_user_repository")]
async fn initialize_user_repository(
    region_string: String,
) -> Result<UserRepositoryImpl, AnyhowError> {
    let client = DynamoDbClient::new(region_string.clone()).await?;
    let table_name = get_env("TABLE_NAME", "Users");
    Ok(UserRepositoryImpl::new(client, table_name))
}

#[instrument(name = "lambda.users.update.check_update_permission")]
fn check_update_permission(user: &User) -> Result<(), AnyhowError> {
    if user.has_permission(Permissions::UPDATE) {
        Ok(())
    } else {
        Err(anyhow!("User does not have UPDATE permission"))
    }
}

#[instrument(name = "lambda.users.update.parse_update_user_request")]
fn parse_update_user_request(body: Option<&str>) -> Result<UpdateUserRequest, AnyhowError> {
    let body_str = body.ok_or_else(|| anyhow!("Missing request body"))?;
    let request: UpdateUserRequest = serde_json::from_str(body_str)
        .map_err(|e| anyhow!("Failed to parse UpdateUserRequest: {}", e))?;
    Ok(request)
}

#[instrument(name = "lambda.users.update.update_user_handler")]
async fn update_user_handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    let (user_id, _) =
        LambdaEventRequestHandler::get_ids_from_request_context(event.clone()).await?;

    let update_user_request = parse_update_user_request(event.payload.body.as_deref())?;
    let region_string = get_env("AWS_REGION", "ap-northeast-1");
    let repository = initialize_user_repository(region_string).await?;

    let mut user = repository.get_user_by_id(user_id.clone()).await?;
    match check_update_permission(&user) {
        Ok(_) => {
            user.name = update_user_request.user_name.clone();
            user.organization_name = update_user_request.organization_name.clone();

            let new_roles = update_user_request.roles.clone();
            if !new_roles.is_empty() {
                user.set_from_roles(new_roles);
            }

            let _ = repository.update_user(user).await?;
            let response = UpdateUserResponse {
                message: format!("User {} has been updated.", user_id),
            };
            Ok(apigw_response(
                200,
                Some(serde_json::to_string(&response)?.into()),
                None,
            ))
        }
        Err(e) => {
            let err_msg = format!("user does not have permission: {:?}", e);
            error!(err_msg);
            Ok(apigw_response(403, Some(err_msg.into()), None))
        }
    }
}

#[instrument(name = "lambda.users.update.handler")]
async fn handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    debug!("handling lambda req: {:?}", event);
    LambdaEventRequestHandler::handle_requests(
        event,
        "/organizations/{organizationId}/users/{userId}",
        update_user_handler,
    )
    .await
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    shared::tracer::init_tracing();
    info!("Starting auth user update function");
    lambda_runtime::run(service_fn(handler)).await
}
