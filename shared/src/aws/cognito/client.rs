use crate::aws::cognito::error::CognitoError;

use aws_config::{meta::region::RegionProviderChain, Region};
use aws_sdk_cognitoidentityprovider::{
    operation::{
        admin_create_user::AdminCreateUserOutput, admin_delete_user::AdminDeleteUserOutput,
        admin_get_user::AdminGetUserOutput, admin_set_user_password::AdminSetUserPasswordOutput,
        admin_update_user_attributes::AdminUpdateUserAttributesOutput,
        initiate_auth::InitiateAuthOutput,
    },
    types::{AttributeType, AuthFlowType, DeliveryMediumType, MessageActionType},
    Client,
};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::sync::Arc;

use tracing::instrument;

pub struct CognitoClient {
    client: Arc<Client>,
    user_pool_id: String,
    client_id: String,
    client_secret: String,
}

impl CognitoClient {
    pub async fn new(
        region_string: String,
        user_pool_id: String,
        client_id: String,
        client_secret: String,
    ) -> Result<Self, CognitoError> {
        let region = Region::new(region_string);
        let region_provider = RegionProviderChain::default_provider().or_else(region);
        let config = aws_config::from_env().region(region_provider).load().await;
        let client = Arc::new(Client::new(&config));
        Ok(CognitoClient {
            client,
            user_pool_id,
            client_id,
            client_secret,
        })
    }

    #[instrument(
        skip(self),
        fields(user_pool_id = %self.user_pool_id, username = %username),
        name = "aws.cognito.admin_create_user"
    )]
    pub async fn admin_create_user(
        &self,
        username: String,
    ) -> Result<AdminCreateUserOutput, CognitoError> {
        let result = self
            .client
            .admin_create_user()
            .user_pool_id(&self.user_pool_id)
            .username(&username)
            .message_action(MessageActionType::Suppress)
            .desired_delivery_mediums(DeliveryMediumType::Email)
            .send()
            .await?;

        Ok(result)
    }

    #[instrument(
        skip(self),
        fields(user_pool_id = %self.user_pool_id, username = %username),
        name = "aws.cognito.admin_delete_user"
    )]
    pub async fn admin_delete_user(
        &self,
        username: String,
    ) -> Result<AdminDeleteUserOutput, CognitoError> {
        let result = self
            .client
            .admin_delete_user()
            .user_pool_id(&self.user_pool_id)
            .username(&username)
            .send()
            .await?;

        Ok(result)
    }

    #[instrument(
        skip(self),
        fields(user_pool_id = %self.user_pool_id, username = %username),
        name = "aws.cognito.admin_get_user"
    )]
    pub async fn admin_get_user(
        &self,
        username: String,
    ) -> Result<AdminGetUserOutput, CognitoError> {
        let result = self
            .client
            .admin_get_user()
            .user_pool_id(&self.user_pool_id)
            .username(&username)
            .send()
            .await?;

        Ok(result)
    }

    #[instrument(
        skip(self, password),
        fields(user_pool_id = %self.user_pool_id, username = %username),
        name = "aws.cognito.admin_set_user_password"
    )]
    pub async fn admin_set_user_password(
        &self,
        username: &str,
        password: &str,
        permanent: bool,
    ) -> Result<AdminSetUserPasswordOutput, CognitoError> {
        let result = self
            .client
            .admin_set_user_password()
            .user_pool_id(&self.user_pool_id)
            .username(username)
            .password(password)
            .permanent(permanent)
            .send()
            .await?;

        Ok(result)
    }

    #[instrument(
        skip(self),
        fields(user_pool_id = %self.user_pool_id, username = %username),
        name = "aws.cognito.email_verified"
    )]
    pub async fn email_verified(
        &self,
        username: String,
    ) -> Result<AdminUpdateUserAttributesOutput, CognitoError> {
        let user_attributes = vec![AttributeType::builder()
            .name("email_verified")
            .value("true")
            .build()?];

        let result = self
            .client
            .admin_update_user_attributes()
            .user_pool_id(&self.user_pool_id)
            .username(&username)
            .set_user_attributes(Some(user_attributes))
            .send()
            .await?;

        Ok(result)
    }

    pub async fn calculate_hash(&self, username: String) -> Result<String, CognitoError> {
        let mut mac = Hmac::<Sha256>::new_from_slice(self.client_secret.as_bytes())
            .map_err(|e| CognitoError::Unknown(e.to_string()))?;
        let message = format!("{}{}", username, self.client_id);
        mac.update(message.as_bytes());
        Ok(STANDARD.encode(mac.finalize().into_bytes()))
    }

    #[instrument(
        skip(self, password, hash),
        fields(user_pool_id = %self.user_pool_id, username = %username, email = %email),
        name = "aws.cognito.user_login"
    )]
    pub async fn user_login(
        &self,
        username: String,
        email: String,
        password: String,
        hash: String,
    ) -> Result<InitiateAuthOutput, CognitoError> {
        let result = self
            .client
            .initiate_auth()
            .client_id(&self.client_id)
            .auth_flow(AuthFlowType::UserPasswordAuth)
            .auth_parameters("USERNAME", &username)
            .auth_parameters("EMAIL", &email)
            .auth_parameters("PASSWORD", &password)
            .auth_parameters("SECRET_HASH", &hash)
            .send()
            .await?;

        Ok(result)
    }

    #[instrument(
        skip(self, hash),
        fields(user_pool_id = %self.user_pool_id, refresh_token = %refresh_token),
        name = "aws.cognito.refresh_token"
    )]
    pub async fn refresh_token(
        &self,
        refresh_token: String,
        hash: String,
    ) -> Result<InitiateAuthOutput, CognitoError> {
        let result = self
            .client
            .initiate_auth()
            .client_id(&self.client_id)
            .auth_flow(AuthFlowType::RefreshToken)
            .auth_parameters("REFRESH_TOKEN", &refresh_token)
            .auth_parameters("SECRET_HASH", &hash)
            .send()
            .await?;

        Ok(result)
    }
}
