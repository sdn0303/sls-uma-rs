use shared::entity::user::Role;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub(super) struct UpdateUserRequest {
    pub user_name: String,
    pub organization_name: String,
    pub roles: Vec<Role>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct UpdateUserResponse {
    pub message: String,
}
