use serde::{Deserialize, Serialize};
use shared::errors::LambdaError;

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct RefreshTokenRequest {
    pub grant_type: String,
    pub refresh_token: String,
}

impl RefreshTokenRequest {
    pub fn validate(&self) -> Result<(), LambdaError> {
        if self.grant_type != "refresh_token" {
            return Err(LambdaError::InvalidRefreshToken);
        }

        if self.refresh_token.is_empty() {
            return Err(LambdaError::InvalidRefreshToken);
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct RefreshTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
}
