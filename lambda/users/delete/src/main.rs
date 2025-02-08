mod requests;

use crate::requests::DeleteUserResponse;

use shared::aws::cognito::client::CognitoClient;
use shared::aws::dynamodb::client::DynamoDbClient;
use shared::aws::lambda_events::{request::LambdaEventRequestHandler, response::apigw_response};
use shared::entity::secrets::Secrets;
use shared::entity::user::{Permissions, User};
use shared::repository::user_repository::{UserRepository, UserRepositoryImpl};
use shared::utils::env::get_env;

use anyhow::{anyhow, Error as AnyhowError};
use aws_lambda_events::event::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};
use lambda_runtime::{service_fn, Error, LambdaEvent};
use tracing::{debug, error, info, instrument};

#[instrument(name = "lambda.users.delete.initialize_user_repository")]
async fn initialize_user_repository(
    region_string: String,
) -> Result<UserRepositoryImpl, AnyhowError> {
    let client = DynamoDbClient::new(region_string.clone()).await?;
    let table_name = get_env("TABLE_NAME", "Users");
    Ok(UserRepositoryImpl::new(client, table_name))
}

#[instrument(name = "lambda.users.delete.initialize_cognito_client")]
async fn initialize_cognito_client(region_string: String) -> Result<CognitoClient, AnyhowError> {
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

#[instrument(name = "lambda.users.delete.check_delete_permission")]
fn check_delete_permission(user: &User) -> Result<(), AnyhowError> {
    if user.has_permission(Permissions::DELETE) {
        Ok(())
    } else {
        Err(anyhow!("User does not have DELETE permission"))
    }
}

#[instrument(name = "lambda.users.get.delete_user_handler")]
async fn delete_user_handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    let (user_id, organization_id) =
        LambdaEventRequestHandler::get_ids_from_request_context(event.clone()).await?;

    let region_string = get_env("REGION", "ap-northeast-1");
    let client = initialize_cognito_client(region_string.clone()).await?;
    let repository = initialize_user_repository(region_string).await?;

    let user = repository.get_user_by_id(user_id.clone()).await?;
    match check_delete_permission(&user) {
        Ok(_) => {
            client.admin_delete_user(user_id.clone()).await?;
            repository
                .delete_user_by_id(user_id.clone(), organization_id)
                .await?;

            let response = DeleteUserResponse {
                message: format!("User {} has been deleted.", user_id),
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

#[tokio::main]
async fn main() -> Result<(), Error> {
    shared::tracer::init_tracing();
    info!("Starting auth user delete function");
    lambda_runtime::run(service_fn(handler)).await
}
