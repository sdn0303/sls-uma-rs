use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct TokenValidateRequest {
    pub token: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct TokenValidateResponse {
    pub user_id: String,
    pub organization_id: String,
}
