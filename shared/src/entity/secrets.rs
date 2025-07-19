use crate::aws::secret_manager::client::SecretManagerClient;
use crate::utils::env::get_env;

use anyhow::{anyhow, Error};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Secrets {
    #[serde(rename = "COGNITO_USER_POOL_ID")]
    pub user_pool_id: String,
    #[serde(rename = "COGNITO_CLIENT_ID")]
    pub client_id: String,
    #[serde(rename = "COGNITO_CLIENT_SECRET")]
    pub client_secret: String,
    #[serde(rename = "COGNITO_JWKS_URL")]
    pub jwks_url: String,
}

impl Secrets {
    pub async fn get_secrets(region: String) -> Result<Self, Error> {
        info!("Setting up Secret Manager client");
        let client = SecretManagerClient::new(region).await?;

        // Get secret name from environment variable
        let secret_name = get_env(
            "COGNITO_SECRET_NAME",
            "dev/UserManagementAuthApi/CognitoEnv",
        );
        info!("Getting secret from: {}", secret_name);

        let secret_output = client.get_secret(&secret_name).await?;

        let secret_string = secret_output
            .secret_string
            .ok_or_else(|| anyhow!("Missing secret string for: {}", secret_name))?;

        // Parse JSON string into Secrets struct
        let secrets: Secrets = serde_json::from_str(&secret_string).map_err(|e| {
            error!("Failed to parse secrets JSON: {}", e);
            anyhow!("Failed to parse secrets JSON: {}", e)
        })?;

        info!("Successfully retrieved and parsed secrets");
        Ok(secrets)
    }
}
