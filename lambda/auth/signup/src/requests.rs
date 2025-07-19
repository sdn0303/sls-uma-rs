use shared::errors::LambdaError;
use shared::utils::regex::{is_valid_username, EMAIL_REGEX};

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub(super) struct SignupRequest {
    pub organization_name: String,
    pub user_name: String,
    pub email: String,
    pub password: String,
}

impl SignupRequest {
    pub fn validate(&self) -> Result<(), LambdaError> {
        // Organization name validation
        if self.organization_name.len() < 2 || self.organization_name.len() > 100 {
            return Err(LambdaError::InvalidOrganizationName);
        }

        // Username validation
        if !is_valid_username(&self.user_name) {
            return Err(LambdaError::InvalidUsername);
        }

        // Email validation
        if !EMAIL_REGEX.is_match(&self.email) {
            return Err(LambdaError::InvalidEmail);
        }

        // Password validation (apply stricter rules)
        if self.password.len() < 8 {
            return Err(LambdaError::InvalidPassword);
        }

        // Password strength check (must contain uppercase, lowercase, and numbers)
        let has_uppercase = self.password.chars().any(|c| c.is_uppercase());
        let has_lowercase = self.password.chars().any(|c| c.is_lowercase());
        let has_digit = self.password.chars().any(|c| c.is_digit(10));

        if !has_uppercase || !has_lowercase || !has_digit {
            return Err(LambdaError::InvalidPassword);
        }

        Ok(())
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub(super) struct SignupResponse {
    pub message: String,
}
