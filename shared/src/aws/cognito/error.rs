use aws_sdk_cognitoidentityprovider::error::{BuildError, SdkError};
use aws_sdk_cognitoidentityprovider::operation::{
    admin_create_user::AdminCreateUserError, admin_delete_user::AdminDeleteUserError,
    admin_get_user::AdminGetUserError, admin_set_user_password::AdminSetUserPasswordError,
    admin_update_user_attributes::AdminUpdateUserAttributesError, initiate_auth::InitiateAuthError,
};
use hmac::digest::InvalidLength as HmacInvalidLength;
use jsonwebtoken::errors::Error as JwtError;
use reqwest::Error as ReqwestError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CognitoError {
    #[error("BuildError: {0}")]
    BuildError(#[from] BuildError),

    #[error("AdminCreateUserError: {0}")]
    AdminCreateUserError(#[from] SdkError<AdminCreateUserError>),

    #[error("AdminDeleteUserError: {0}")]
    AdminDeleteUserError(#[from] SdkError<AdminDeleteUserError>),

    #[error("AdminGetUserError: {0}")]
    AdminGetUserError(#[from] SdkError<AdminGetUserError>),

    #[error("AdminSetUserPasswordError: {0}")]
    AdminSetUserPasswordError(#[from] SdkError<AdminSetUserPasswordError>),

    #[error("AdminUpdateUserAttributesError: {0}")]
    AdminUpdateUserAttributesError(#[from] SdkError<AdminUpdateUserAttributesError>),

    #[error("InitiateAuthError: {0}")]
    InitiateAuthError(#[from] SdkError<InitiateAuthError>),

    #[error("JWT Error: {0}")]
    JwtError(#[from] JwtError),

    #[error("HMAC Invalid Length: {0}")]
    HmacInvalidLength(#[from] HmacInvalidLength),

    #[error("Reqwest Error: {0}")]
    ReqwestError(#[from] ReqwestError),

    #[error("Http Error: {0}")]
    HttpError(String),

    #[error("Invalid Token Error: {0}")]
    InvalidTokenError(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}
