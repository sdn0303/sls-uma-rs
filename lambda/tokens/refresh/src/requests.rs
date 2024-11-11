use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct RefreshTokenRequest {
    pub grant_type: String,
    pub refresh_token: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RefreshTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
}
