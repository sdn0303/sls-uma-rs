use crate::aws::secret_manager::error::SecretManagerError;

use anyhow::Result;
use aws_config::{meta::region::RegionProviderChain, Region};
use aws_sdk_secretsmanager::{operation::get_secret_value::GetSecretValueOutput, Client};
use futures::future::try_join_all;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::instrument;

const DEFAULT_CONCURRENCY_LIMIT: usize = 5;

pub struct SecretManagerClient {
    client: Arc<Client>,
}

impl SecretManagerClient {
    pub async fn new(region_string: String) -> Result<Self, SecretManagerError> {
        let region = Region::new(region_string);
        let region_provider = RegionProviderChain::default_provider().or_else(region);
        let config = aws_config::from_env().region(region_provider).load().await;
        let client = Arc::new(Client::new(&config));
        Ok(SecretManagerClient { client })
    }

    #[instrument(skip(self), fields(secret = %secret_name), name = "aws.secret_manager.get_secret")]
    pub async fn get_secret(
        &self,
        secret_name: &str,
    ) -> Result<GetSecretValueOutput, SecretManagerError> {
        let secret = self
            .client
            .get_secret_value()
            .secret_id(secret_name)
            .send()
            .await
            .map_err(|e| SecretManagerError::GetSecretValueError(Box::new(e)))?;

        Ok(secret)
    }

    #[instrument(
        skip(self, semaphore),
        fields(key = %key),
        name = "aws.secret_manager.get_secret_string"
    )]
    async fn get_secret_string(
        &self,
        key: &str,
        semaphore: Arc<Semaphore>,
    ) -> Result<(String, String), SecretManagerError> {
        let _permit = semaphore.acquire().await.map_err(|e| {
            SecretManagerError::SemaphoreError(format!(
                "Failed to acquire semaphore for key {}: {}",
                key, e
            ))
        })?;

        let secret_response = self.get_secret(key).await?;
        let secret = secret_response.secret_string.ok_or_else(|| {
            SecretManagerError::MissingAttribute(format!("Missing secret string for key: {}", key))
        })?;

        Ok((key.to_string(), secret))
    }

    #[instrument(
        skip(self, secret_keys),
        fields(secret_count = secret_keys.len()),
        name = "aws.secret_manager.get_secrets"
    )]
    pub async fn get_secrets<I>(
        &self,
        secret_keys: I,
    ) -> Result<HashMap<String, String>, SecretManagerError>
    where
        I: IntoIterator<Item = String> + ExactSizeIterator,
    {
        let secret_keys: Vec<String> = secret_keys.into_iter().collect();
        let semaphore = Arc::new(Semaphore::new(DEFAULT_CONCURRENCY_LIMIT));

        let fetch_futures = secret_keys.iter().map(|key| {
            let client = <&SecretManagerClient>::clone(&self);
            let key = key.clone();
            let semaphore = semaphore.clone();
            async move { client.get_secret_string(&key, semaphore).await }
        });

        let results = try_join_all(fetch_futures).await?;
        let hashmap: HashMap<String, String> = results.into_iter().collect();

        Ok(hashmap)
    }
}
