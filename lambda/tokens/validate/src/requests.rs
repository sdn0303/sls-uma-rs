use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct TokenValidateRequest {
    pub token: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TokenValidateResponse {
    pub user_id: String,
    pub organization_id: String,
}
