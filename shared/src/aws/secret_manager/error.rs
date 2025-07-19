use aws_sdk_secretsmanager::{
    error::{BuildError, SdkError},
    operation::get_secret_value::GetSecretValueError,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SecretManagerError {
    #[error("GetSecretValueError: {0}")]
    GetSecretValueError(#[from] Box<SdkError<GetSecretValueError>>),

    #[error("BuildError: {0}")]
    BuildError(BuildError),

    #[error("SemaphoreError: {0}")]
    SemaphoreError(String),

    #[error("Not found")]
    NotFound,

    #[error("MissingAttribute: {0}")]
    MissingAttribute(String),

    #[error("InvalidAttribute: {0}")]
    InvalidAttribute(String),

    #[error("Other: {0}")]
    Other(String),
}
