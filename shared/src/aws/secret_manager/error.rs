use aws_sdk_secretsmanager::{error::SdkError, operation::get_secret_value::GetSecretValueError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SecretManagerError {
    #[error("GetSecretValueError: {0}")]
    GetSecretValueError(#[from] SdkError<GetSecretValueError>),

    #[error("Semaphore error: {0}")]
    SemaphoreError(String),

    #[error("Missing secret string: {0}")]
    MissingSecretString(String),

    #[error("Other error: {0}")]
    Other(String),
}
