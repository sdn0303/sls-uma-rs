use serde::{Deserialize, Serialize};
use shared::errors::LambdaError;

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct TokenValidateRequest {
    pub token: String,
}

impl TokenValidateRequest {
    pub fn validate(&self) -> Result<(), LambdaError> {
        if self.token.is_empty() {
            return Err(LambdaError::MissingToken);
        }

        // Basic JWT format validation (3 parts separated by dots)
        let parts: Vec<&str> = self.token.split('.').collect();
        if parts.len() != 3 {
            return Err(LambdaError::InvalidToken);
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct TokenValidateResponse {
    pub user_id: String,
    pub organization_id: String,
}
