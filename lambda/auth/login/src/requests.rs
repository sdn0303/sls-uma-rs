use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct LoginResponse {
    pub access_token: String,
    pub id_token: String,
    pub refresh_token: String,
}
