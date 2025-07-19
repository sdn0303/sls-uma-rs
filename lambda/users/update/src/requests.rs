use shared::entity::user::Role;
use shared::errors::LambdaError;
use shared::utils::regex::is_valid_username;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(super) struct UpdateUserRequest {
    pub user_name: String,
    pub organization_name: String,
    pub roles: Vec<Role>,
}

impl UpdateUserRequest {
    pub fn validate(&self) -> Result<(), LambdaError> {
        // Username validation
        if !is_valid_username(&self.user_name) {
            return Err(LambdaError::InvalidUsername);
        }

        // Organization name validation
        if self.organization_name.len() < 2 || self.organization_name.len() > 100 {
            return Err(LambdaError::InvalidOrganizationName);
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(super) struct UpdateUserResponse {
    pub message: String,
}
