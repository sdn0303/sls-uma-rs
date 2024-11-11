use crate::aws::secret_manager::client::SecretManagerClient;
use crate::utils::env::get_env;

use anyhow::{anyhow, Error};
use std::collections::HashMap;
use tracing::{error, info};

pub struct Secrets {
    pub user_pool_id: String,
    pub client_id: String,
    pub client_secret: String,
    pub jwks_url: String,
}

impl Secrets {
    pub async fn get_secrets(region_string: String) -> Result<Secrets, Error> {
        info!("setup secret manager client");
        let client = SecretManagerClient::new(region_string).await?;
        let secret_keys = vec![
            get_env("COGNITO_USER_POOL_ID", ""),
            get_env("COGNITO_CLIENT_ID", ""),
            get_env("COGNITO_CLIENT_SECRET", ""),
            get_env("COGNITO_JWKS_URL", ""),
        ];

        info!("start to get secrets");
        let secrets_map: HashMap<String, String> =
            client.get_secrets(secret_keys.into_iter()).await?;
        match Self::from_secrets_map(secrets_map).await {
            Ok(secrets) => Ok(secrets),
            Err(err) => {
                error!("failed to get secrets: {}", err);
                Err(anyhow!("failed to get secrets: {:?}", err))
            }
        }
    }
    pub async fn from_secrets_map(secrets: HashMap<String, String>) -> Result<Self, Error> {
        Ok(Secrets {
            user_pool_id: secrets
                .get("COGNITO_USER_POOL_ID")
                .cloned()
                .ok_or_else(|| anyhow!("Missing cognito_user_pool_id"))?,
            client_id: secrets
                .get("cognito_client_id")
                .cloned()
                .ok_or_else(|| anyhow!("Missing cognito_client_id"))?,
            client_secret: secrets
                .get("cognito_client_secret")
                .cloned()
                .ok_or_else(|| anyhow!("Missing cognito_client_secret"))?,
            jwks_url: secrets
                .get("")
                .cloned()
                .ok_or_else(|| anyhow!("Missing jwks_url"))?,
        })
    }
}
