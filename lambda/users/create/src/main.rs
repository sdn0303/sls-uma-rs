mod requests;

use crate::requests::{CreateUserRequest, CreateUserResponse};

use shared::aws::cognito::client::CognitoClient;
use shared::aws::dynamodb::client::DynamoDbClient;
use shared::aws::lambda_events::request::LambdaEventRequestHandler;
use shared::aws::lambda_events::response::apigw_response;
use shared::entity::secrets::Secrets;
use shared::entity::user::{Permissions, Role, User};
use shared::repository::user_repository::{UserRepository, UserRepositoryImpl};
use shared::utils::{env::get_env, password::generate_password};

use anyhow::{anyhow, Error as AnyhowError};
use aws_lambda_events::event::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};
use lambda_runtime::{service_fn, Error, LambdaEvent};
use std::collections::HashSet;
use tracing::{debug, error, info, instrument};

#[instrument(name = "lambda.users.create.initialize_user_repository")]
async fn initialize_user_repository(
    region_string: String,
) -> Result<UserRepositoryImpl, AnyhowError> {
    let client = DynamoDbClient::new(region_string.clone()).await?;
    let table_name = get_env("TABLE_NAME", "Users");
    Ok(UserRepositoryImpl::new(client, table_name))
}

#[instrument(name = "lambda.users.create.initialize_cognito_client")]
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

#[instrument(name = "lambda.users.create.check_create_permission")]
fn check_create_permission(user: &User) -> Result<(), AnyhowError> {
    if user.has_permission(Permissions::CREATE) {
        Ok(())
    } else {
        Err(anyhow!("User does not have CREATE permission"))
    }
}

#[instrument(name = "lambda.users.create.parse_create_user_request")]
fn parse_create_user_request(body: Option<&str>) -> Result<CreateUserRequest, AnyhowError> {
    let body_str = body.ok_or_else(|| anyhow!("Missing request body"))?;
    let request: CreateUserRequest = serde_json::from_str(body_str)
        .map_err(|e| anyhow!("Failed to parse CreateUserRequest: {}", e))?;
    Ok(request)
}

#[instrument(name = "lambda.users.create.generate_new_user")]
fn generate_new_user(id: String, request: CreateUserRequest) -> Result<User, AnyhowError> {
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

#[instrument(name = "lambda.users.create.build_create_user_response")]
fn build_create_user_response(
    user: &User,
    tmp_password: String,
) -> Result<CreateUserResponse, AnyhowError> {
    let roles = user.roles.iter().cloned().collect::<Vec<Role>>();
    Ok(CreateUserResponse {
        user_name: user.name.clone(),
        user_email: user.email.clone(),
        user_roles: roles,
        user_tmp_password: tmp_password,
    })
}

#[instrument(name = "lambda.users.create.create_user_handler")]
async fn create_user_handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    let (user_id, _) =
        LambdaEventRequestHandler::get_ids_from_request_context(event.clone()).await?;

    let region_string = get_env("REGION", "ap-northeast-1");
    let client = initialize_cognito_client(region_string.clone()).await?;
    let repository = initialize_user_repository(region_string).await?;

    let create_user_request = parse_create_user_request(event.payload.body.as_deref())?;
    let user = repository.get_user_by_id(user_id.clone()).await?;
    match check_create_permission(&user) {
        Ok(_) => {
            let tmp_password = generate_password()?;
            debug!("Password has been generated: {:?}", tmp_password);

            let admin_create_user_opt = client.admin_create_user(user.email.clone()).await?;
            debug!("admin create user output: {:?}", admin_create_user_opt);

            let opt = client
                .admin_set_user_password(&user.email.clone(), &tmp_password, true)
                .await?;
            debug!("admin set user password output: {:?}", opt);

            let opt = client.email_verified(user.email.clone()).await?;
            debug!("email verified user output: {:?}", opt);

            let sub = admin_create_user_opt
                .user()
                .ok_or("user is None")?
                .attributes()
                .iter()
                .find(|attr| attr.name() == "sub")
                .ok_or("sub is None")?
                .value()
                .ok_or("sub value is None")?;
            let tmp_password = generate_password()?;
            debug!("Generated new password for user {}", user.name);

            let new_user = generate_new_user(sub.to_string(), create_user_request)?;
            let created_user = repository.create_user(new_user).await?;
            let response = build_create_user_response(&created_user, tmp_password)?;
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

#[tokio::main]
async fn main() -> Result<(), Error> {
    shared::tracer::init_tracing();
    info!("Starting auth user create function");
    lambda_runtime::run(service_fn(handler)).await
}
