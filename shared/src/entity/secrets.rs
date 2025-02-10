use crate::aws::secret_manager::client::SecretManagerClient;

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
    pub async fn get_secrets(region: String) -> Result<Self, Error> {
        info!("Setting up Secret Manager client");
        let client = SecretManagerClient::new(region).await?;
        let secret_keys = [
            "COGNITO_USER_POOL_ID",
            "COGNITO_CLIENT_ID",
            "COGNITO_CLIENT_SECRET",
            "COGNITO_JWKS_URL",
        ];

        info!("Starting to get secrets");
        let secrets_map: HashMap<String, String> = client
            .get_secrets(secret_keys.iter().map(|s| s.to_string()))
            .await?;
        Secrets::from_secrets_map(&secrets_map).map_err(|err| {
            error!("failed to get secrets: {}", err);
            anyhow!("failed to get secrets: {:?}", err)
        })
    }

    fn get_value(map: &HashMap<String, String>, key: &str) -> Result<String, Error> {
        map.get(key)
            .cloned()
            .ok_or_else(|| anyhow!("Missing secret: {}", key))
    }

    fn from_secrets_map(secrets: &HashMap<String, String>) -> Result<Self, Error> {
        Ok(Secrets {
            user_pool_id: Self::get_value(secrets, "COGNITO_USER_POOL_ID")?,
            client_id: Self::get_value(secrets, "COGNITO_CLIENT_ID")?,
            client_secret: Self::get_value(secrets, "COGNITO_CLIENT_SECRET")?,
            jwks_url: Self::get_value(secrets, "COGNITO_JWKS_URL")?,
        })
    }
}
