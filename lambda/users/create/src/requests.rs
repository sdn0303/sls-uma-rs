use shared::entity::user::Role;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateUserRequest {
    pub user_name: String,
    pub email: String,
    pub organization_id: String,
    pub organization_name: String,
    pub roles: Vec<Role>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateUserResponse {
    pub user_name: String,
    pub user_email: String,
    pub user_roles: Vec<Role>,
    pub user_tmp_password: String,
}
