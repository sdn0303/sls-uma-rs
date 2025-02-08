use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub(super) struct SignupRequest {
    pub organization_name: String,
    pub user_name: String,
    pub email: String,
    pub password: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub(super) struct SignupResponse {
    pub message: String,
}
