use shared::entity::user::Role;
use shared::errors::LambdaError;
use shared::utils::regex::{is_valid_username, EMAIL_REGEX};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(super) struct CreateUserRequest {
    pub user_name: String,
    pub email: String,
    pub organization_id: String,
    pub organization_name: String,
    pub roles: Vec<Role>,
}

impl CreateUserRequest {
    pub fn validate(&self) -> Result<(), LambdaError> {
        // Username validation
        if !is_valid_username(&self.user_name) {
            return Err(LambdaError::InvalidUsername);
        }

        // Email validation
        if !EMAIL_REGEX.is_match(&self.email) {
            return Err(LambdaError::InvalidEmail);
        }

        // Organization ID validation
        if self.organization_id.is_empty() {
            return Err(LambdaError::MissingOrganizationId);
        }

        // Organization name validation
        if self.organization_name.len() < 2 || self.organization_name.len() > 100 {
            return Err(LambdaError::InvalidOrganizationName);
        }

        // Role validation
        if self.roles.is_empty() {
            return Err(LambdaError::MissingRoles);
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(super) struct CreateUserResponse {
    pub user_name: String,
    pub user_email: String,
    pub user_roles: Vec<Role>,
    pub user_tmp_password: String,
}
