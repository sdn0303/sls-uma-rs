use crate::aws::cognito::client::CognitoClient;
use crate::aws::cognito::token_authorizer::CognitoTokenAuthorizer;
use crate::aws::dynamodb::client::DynamoDbClient;
use crate::entity::secrets::Secrets;
use crate::errors::LambdaResult;

use async_trait::async_trait;
use std::sync::Arc;

/// Trait for managing Cognito client instances
#[async_trait]
pub trait CognitoClientManager {
    async fn get_client(&self) -> LambdaResult<CognitoClient>;
}

/// Trait for managing DynamoDB client instances
#[async_trait]
pub trait DynamoDbClientManager {
    async fn get_client(&self) -> LambdaResult<Arc<DynamoDbClient>>;
}

/// Trait for managing Cognito token authorizer instances
#[async_trait]
pub trait TokenAuthorizerManager {
    async fn get_authorizer(&self) -> LambdaResult<CognitoTokenAuthorizer>;
}

/// Trait for managing secrets
#[async_trait]
pub trait SecretsManager {
    async fn get_secrets(&self) -> LambdaResult<Secrets>;
}

/// Default implementation using global instances
pub struct DefaultClientManager {
    region: String,
}

impl DefaultClientManager {
    pub fn new(region: String) -> Self {
        Self { region }
    }
}

#[async_trait]
impl CognitoClientManager for DefaultClientManager {
    async fn get_client(&self) -> LambdaResult<CognitoClient> {
        // This will be implemented to use the global instance
        // but with better error handling and abstraction
        let secrets = Secrets::get_secrets(self.region.clone())
            .await
            .map_err(|e| crate::errors::LambdaError::InternalError(e.to_string()))?;

        CognitoClient::new(
            self.region.clone(),
            secrets.user_pool_id,
            secrets.client_id,
            secrets.client_secret,
        )
        .await
        .map_err(|e| crate::errors::LambdaError::InternalError(e.to_string()))
    }
}

#[async_trait]
impl DynamoDbClientManager for DefaultClientManager {
    async fn get_client(&self) -> LambdaResult<Arc<DynamoDbClient>> {
        DynamoDbClient::new(self.region.clone())
            .await
            .map(Arc::new)
            .map_err(|e| crate::errors::LambdaError::InternalError(e.to_string()))
    }
}

#[async_trait]
impl TokenAuthorizerManager for DefaultClientManager {
    async fn get_authorizer(&self) -> LambdaResult<CognitoTokenAuthorizer> {
        let secrets = Secrets::get_secrets(self.region.clone())
            .await
            .map_err(|e| crate::errors::LambdaError::InternalError(e.to_string()))?;

        Ok(
            CognitoTokenAuthorizer::new(
                secrets.user_pool_id,
                secrets.jwks_url,
                self.region.clone(),
            )
            .await,
        )
    }
}

#[async_trait]
impl SecretsManager for DefaultClientManager {
    async fn get_secrets(&self) -> LambdaResult<Secrets> {
        Secrets::get_secrets(self.region.clone())
            .await
            .map_err(|e| crate::errors::LambdaError::InternalError(e.to_string()))
    }
}

/// Mock implementation for testing
#[cfg(test)]
pub struct MockClientManager {
    pub cognito_client: Option<CognitoClient>,
    pub dynamodb_client: Option<Arc<DynamoDbClient>>,
    pub token_authorizer: Option<CognitoTokenAuthorizer>,
    pub secrets: Option<Secrets>,
}

#[cfg(test)]
#[async_trait]
impl CognitoClientManager for MockClientManager {
    async fn get_client(&self) -> LambdaResult<CognitoClient> {
        self.cognito_client.clone().ok_or_else(|| {
            crate::errors::LambdaError::InternalError("Mock client not set".to_string())
        })
    }
}

#[cfg(test)]
#[async_trait]
impl DynamoDbClientManager for MockClientManager {
    async fn get_client(&self) -> LambdaResult<Arc<DynamoDbClient>> {
        self.dynamodb_client.clone().ok_or_else(|| {
            crate::errors::LambdaError::InternalError("Mock client not set".to_string())
        })
    }
}

#[cfg(test)]
#[async_trait]
impl TokenAuthorizerManager for MockClientManager {
    async fn get_authorizer(&self) -> LambdaResult<CognitoTokenAuthorizer> {
        self.token_authorizer.clone().ok_or_else(|| {
            crate::errors::LambdaError::InternalError("Mock authorizer not set".to_string())
        })
    }
}

#[cfg(test)]
#[async_trait]
impl SecretsManager for MockClientManager {
    async fn get_secrets(&self) -> LambdaResult<Secrets> {
        self.secrets.clone().ok_or_else(|| {
            crate::errors::LambdaError::InternalError("Mock secrets not set".to_string())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client_manager::{
        CognitoClientManager, DynamoDbClientManager, SecretsManager, TokenAuthorizerManager,
    };

    fn create_test_secrets() -> Secrets {
        Secrets {
            user_pool_id: "test-user-pool".to_string(),
            client_id: "test-client-id".to_string(),
            client_secret: "test-client-secret".to_string(),
            jwks_url: "https://test.jwks.url".to_string(),
        }
    }

    #[tokio::test]
    async fn test_mock_client_manager_cognito_client() {
        let test_secrets = create_test_secrets();

        // Create mock client (this would normally require actual AWS credentials)
        let mock_manager = MockClientManager {
            cognito_client: None,
            dynamodb_client: None,
            token_authorizer: None,
            secrets: Some(test_secrets.clone()),
        };

        // Test that getting client fails when not set
        let result = CognitoClientManager::get_client(&mock_manager).await;
        assert!(result.is_err());

        // Note: In a real test, we would set a mock client here
        // mock_manager.cognito_client = Some(mock_client);
        // let result = mock_manager.get_client().await;
        // assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_client_manager_dynamodb_client() {
        let mock_manager = MockClientManager {
            cognito_client: None,
            dynamodb_client: None,
            token_authorizer: None,
            secrets: None,
        };

        // Test that getting client fails when not set
        let result = DynamoDbClientManager::get_client(&mock_manager).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_client_manager_token_authorizer() {
        let mock_manager = MockClientManager {
            cognito_client: None,
            dynamodb_client: None,
            token_authorizer: None,
            secrets: None,
        };

        // Test that getting authorizer fails when not set
        let result = mock_manager.get_authorizer().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_client_manager_secrets() {
        let test_secrets = create_test_secrets();

        let mock_manager = MockClientManager {
            cognito_client: None,
            dynamodb_client: None,
            token_authorizer: None,
            secrets: Some(test_secrets.clone()),
        };

        let result = mock_manager.get_secrets().await;
        assert!(result.is_ok());

        let retrieved_secrets = result.unwrap();
        assert_eq!(retrieved_secrets.user_pool_id, test_secrets.user_pool_id);
        assert_eq!(retrieved_secrets.client_id, test_secrets.client_id);
        assert_eq!(retrieved_secrets.client_secret, test_secrets.client_secret);
        assert_eq!(retrieved_secrets.jwks_url, test_secrets.jwks_url);
    }

    #[tokio::test]
    async fn test_mock_client_manager_secrets_not_set() {
        let mock_manager = MockClientManager {
            cognito_client: None,
            dynamodb_client: None,
            token_authorizer: None,
            secrets: None,
        };

        let result = mock_manager.get_secrets().await;
        assert!(result.is_err());

        if let Err(crate::errors::LambdaError::InternalError(message)) = result {
            assert_eq!(message, "Mock secrets not set");
        } else {
            panic!("Expected InternalError with specific message");
        }
    }

    #[test]
    fn test_default_client_manager_creation() {
        let region = "ap-northeast-1".to_string();
        let manager = DefaultClientManager::new(region.clone());
        assert_eq!(manager.region, region);
    }

    #[test]
    fn test_mock_client_manager_creation() {
        let mock_manager = MockClientManager {
            cognito_client: None,
            dynamodb_client: None,
            token_authorizer: None,
            secrets: None,
        };

        assert!(mock_manager.cognito_client.is_none());
        assert!(mock_manager.dynamodb_client.is_none());
        assert!(mock_manager.token_authorizer.is_none());
        assert!(mock_manager.secrets.is_none());
    }
}
