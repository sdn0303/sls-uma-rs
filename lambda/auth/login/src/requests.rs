use shared::errors::LambdaError;
use shared::utils::regex::EMAIL_REGEX;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub(super) struct LoginRequest {
    pub email: String,
    pub password: String,
}

impl LoginRequest {
    pub fn validate(&self) -> Result<(), LambdaError> {
        // Email validation
        if !EMAIL_REGEX.is_match(&self.email) {
            return Err(LambdaError::InvalidEmail);
        }

        // Password validation
        if self.password.len() < 8 {
            return Err(LambdaError::InvalidPassword);
        }

        Ok(())
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub(super) struct LoginResponse {
    pub access_token: String,
    pub id_token: String,
    pub refresh_token: String,
    pub user_id: String,
    pub organization_id: String,
}
