use aws_sdk_dynamodb::{
    error::{BuildError, SdkError},
    operation::{
        delete_item::DeleteItemError, get_item::GetItemError, put_item::PutItemError,
        query::QueryError, scan::ScanError, update_item::UpdateItemError,
    },
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DynamoDbError {
    #[error("BuildError: {0}")]
    BuildError(BuildError),

    #[error("GetItemError: {0}")]
    GetItemError(#[from] SdkError<GetItemError>),

    #[error("PutItemError: {0}")]
    PutItemError(#[from] SdkError<PutItemError>),

    #[error("UpdateItemError: {0}")]
    UpdateItemError(#[from] SdkError<UpdateItemError>),

    #[error("DeleteItemError: {0}")]
    DeleteItemError(#[from] SdkError<DeleteItemError>),

    #[error("ScanError: {0}")]
    ScanError(#[from] SdkError<ScanError>),

    #[error("QueryError: {0}")]
    QueryError(#[from] SdkError<QueryError>),

    #[error("Not found")]
    NotFound,

    #[error("MissingAttribute: {0}")]
    MissingAttribute(String),

    #[error("InvalidAttribute: {0}")]
    InvalidAttribute(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}
