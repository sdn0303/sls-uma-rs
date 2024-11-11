mod requests;

use crate::requests::ListUsersResponse;
use shared::aws::dynamodb::client::DynamoDbClient;
use shared::aws::lambda_events::{request::LambdaEventRequestHandler, response::apigw_response};
use shared::repository::user_repository::{UserRepository, UserRepositoryImpl};
use shared::utils::env::get_env;

use anyhow::Error as AnyhowError;
use aws_lambda_events::event::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};
use lambda_runtime::{service_fn, Error, LambdaEvent};
use tracing::{debug, info, instrument};

#[instrument(name = "lambda.users.get.initialize_user_repository")]
async fn initialize_user_repository(
    region_string: String,
) -> Result<UserRepositoryImpl, AnyhowError> {
    let client = DynamoDbClient::new(region_string.clone()).await?;
    let table_name = get_env("TABLE_NAME", "Users");
    Ok(UserRepositoryImpl::new(client, table_name))
}

#[instrument(name = "lambda.users.get.get_user_handler")]
async fn get_user_handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    let (user_id, _) =
        LambdaEventRequestHandler::get_ids_from_request_context(event.clone()).await?;
    let region_string = get_env("AWS_REGION", "ap-northeast-1");
    let repository = initialize_user_repository(region_string).await?;
    let user = repository.get_user_by_id(user_id.clone()).await?;
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
    let (_, organization_id) =
        LambdaEventRequestHandler::get_ids_from_request_context(event.clone()).await?;
    let region_string = get_env("AWS_REGION", "ap-northeast-1");
    let repository = initialize_user_repository(region_string).await?;
    let users = repository
        .get_users_by_organization_id(organization_id)
        .await?;
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

#[tokio::main]
async fn main() -> Result<(), Error> {
    shared::tracer::init_tracing();
    info!("Starting auth user get function");
    lambda_runtime::run(service_fn(handler)).await
}
