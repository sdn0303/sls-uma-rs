use crate::aws::cognito::error::CognitoError;

use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{error, info, instrument};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub iss: String,
    pub iat: u64,
    pub exp: u64,
}

pub struct CognitoTokenAuthorizer {
    user_pool_id: String,
    jwks_url: String,
    region: String,
    jwks_cache: Arc<RwLock<(Value, Instant)>>,
}

impl CognitoTokenAuthorizer {
    pub async fn new(user_pool_id: String, jwks_url: String, region: String) -> Self {
        CognitoTokenAuthorizer {
            user_pool_id,
            jwks_url,
            region,
            jwks_cache: Arc::new(RwLock::new((serde_json::json!({}), Instant::now()))),
        }
    }

    async fn get_jwks(&self) -> Result<Value, CognitoError> {
        let mut cache = self.jwks_cache.write().await;
        let now = Instant::now();
        if now.duration_since(cache.1) > Duration::from_secs(3600) {
            info!("Fetching new JWKS from {}", self.jwks_url);
            let client = reqwest::Client::new();
            let response = client.get(&self.jwks_url).send().await.map_err(|e| {
                error!("Failed to fetch JWKS: {:?}", e);
                CognitoError::ReqwestError(e)
            })?;

            if !response.status().is_success() {
                error!("Failed to fetch JWKS: HTTP {}", response.status());
                CognitoError::HttpError(format!(
                    "Failed to fetch JWKS: HTTP {}",
                    response.status()
                ));
            }

            let jwks: Value = response.json().await.map_err(|e| {
                error!("Failed to parse JWKS JSON: {:?}", e);
                CognitoError::ReqwestError(e)
            })?;

            *cache = (jwks, now);
            Ok(cache.0.clone())
        } else {
            info!("Using cached JWKS");
            Ok(cache.0.clone())
        }
    }

    #[instrument(
        skip(self, token),
        fields(user_pool_id = %self.user_pool_id),
        name = "aws.cognito.token_authorizer.validate_token"
    )]
    pub async fn validate_token(&self, token: &str) -> Result<Claims, CognitoError> {
        let jwks = self.get_jwks().await?;

        let header = decode_header(token).map_err(|e| {
            error!("Failed to decode token header: {:?}", e);
            CognitoError::JwtError(e)
        })?;

        let kid = header.kid.ok_or_else(|| {
            error!("Token header missing 'kid'");
            CognitoError::InvalidTokenError("Missing kid".to_string())
        })?;

        info!("Token 'kid' extracted: {}", kid);

        let keys = jwks["keys"].as_array().ok_or_else(|| {
            error!("JWKS does not contain 'keys' array");
            CognitoError::InvalidTokenError("Missing keys".to_string())
        })?;

        let jwk = keys
            .iter()
            .find(|key| key["kid"].as_str() == Some(&kid))
            .ok_or_else(|| {
                error!("No matching JWK found for kid: {}", kid);
                CognitoError::InvalidTokenError("Key not found".to_string())
            })?;

        info!("Matching JWK found for kid: {}", kid);

        let n = jwk["n"].as_str().ok_or_else(|| {
            error!("JWK missing 'n' parameter");
            CognitoError::InvalidTokenError("Missing n".to_string())
        })?;
        let e = jwk["e"].as_str().ok_or_else(|| {
            error!("JWK missing 'e' parameter");
            CognitoError::InvalidTokenError("Missing e".to_string())
        })?;

        let decoding_key = DecodingKey::from_rsa_components(n, e).map_err(|e| {
            error!("Failed to create DecodingKey: {:?}", e);
            CognitoError::JwtError(e)
        })?;

        info!("DecodingKey successfully created");
        let issuer = format!(
            "https://cognito-idp.{}.amazonaws.com/{}",
            self.region, self.user_pool_id
        );
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[issuer.clone()]);

        info!("Validation configured with issuer: {}", issuer);

        let token_data = decode::<Claims>(token, &decoding_key, &validation).map_err(|e| {
            error!("Failed to decode token: {:?}", e);
            CognitoError::JwtError(e)
        })?;

        info!("Token successfully decoded and validated");

        Ok(token_data.claims)
    }
}
