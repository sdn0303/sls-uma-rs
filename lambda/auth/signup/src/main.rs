mod requests;

use crate::requests::{SignupRequest, SignupResponse};
use shared::aws::cognito::client::CognitoClient;
use shared::aws::dynamodb::client::DynamoDbClient;
use shared::aws::lambda_events::{request::LambdaEventRequestHandler, response::apigw_response};
use shared::entity::{
    secrets::Secrets,
    user::{Role, User},
};
use shared::repository::user_repository::{UserRepository, UserRepositoryImpl};
use shared::utils::env::get_env;
use shared::utils::uuid::generate_uuid;

use anyhow::{anyhow, Error as AnyhowError};
use aws_lambda_events::event::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};
use lambda_runtime::{service_fn, Error, LambdaEvent};
use std::collections::HashSet;
use tracing::{debug, info, instrument};

#[instrument(name = "lambda.auth.signup.initialize_user_repository")]
async fn initialize_user_repository(
    region_string: String,
) -> Result<UserRepositoryImpl, AnyhowError> {
    let client = DynamoDbClient::new(region_string.clone()).await?;
    let table_name = get_env("TABLE_NAME", "Users");
    Ok(UserRepositoryImpl::new(client, table_name))
}

#[instrument(name = "lambda.auth.signup.initialize_cognito_client")]
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

#[instrument(name = "lambda.auth.signup.parse_signup_request")]
fn parse_signup_request(body: Option<&str>) -> Result<SignupRequest, AnyhowError> {
    let body_str = body.ok_or_else(|| anyhow!("Missing request body"))?;
    let request: SignupRequest = serde_json::from_str(body_str)
        .map_err(|e| anyhow!("Failed to parse SignupRequest: {}", e))?;
    Ok(request)
}

#[instrument(name = "lambda.auth.signup.generate_user")]
fn generate_new_user(id: String, request: SignupRequest) -> Result<User, AnyhowError> {
    let organization_id = generate_uuid();
    let mut roles = HashSet::new();
    roles.insert(Role::Admin);
    let user = User::new(
        id,
        request.user_name,
        request.email,
        organization_id,
        request.organization_name,
        roles,
    );
    Ok(user)
}

#[instrument(name = "lambda.auth.signup.signup_handler")]
async fn signup_handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    let signup_request = parse_signup_request(event.payload.body.as_deref())?;
    let region_string = get_env("AWS_REGION", "ap-northeast-1");
    let client = initialize_cognito_client(region_string.clone()).await?;
    let repository = initialize_user_repository(region_string).await?;

    let admin_create_user_opt = client
        .admin_create_user(signup_request.email.clone())
        .await?;
    debug!("admin create user output: {:?}", admin_create_user_opt);

    let opt = client
        .admin_set_user_password(
            &signup_request.email.clone(),
            &signup_request.password.clone(),
            true,
        )
        .await?;
    debug!("admin set user password output: {:?}", opt);

    let opt = client.email_verified(signup_request.email.clone()).await?;
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
    let new_user = generate_new_user(sub.to_string(), signup_request)?;
    let _ = repository.create_user(new_user).await?;
    let response = SignupResponse {
        message: "signup successfully.".to_string(),
    };
    Ok(apigw_response(
        200,
        Some(serde_json::to_string(&response)?.into()),
        None,
    ))
}

#[instrument(name = "lambda.auth.signup.handler")]
async fn handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    debug!("handling lambda req: {:?}", event);
    LambdaEventRequestHandler::handle_requests(event, "/users", signup_handler).await
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    shared::tracer::init_tracing();
    info!("Starting auth user signup function");
    lambda_runtime::run(service_fn(handler)).await
}
