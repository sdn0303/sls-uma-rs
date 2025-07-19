use thiserror::Error;

/// Unified error type for all Lambda functions
#[derive(Error, Debug)]
pub enum LambdaError {
    // Validation errors
    #[error("Invalid email format")]
    InvalidEmail,
    #[error("Invalid username format (3-30 alphanumeric characters, _ or -)")]
    InvalidUsername,
    #[error("Invalid password format")]
    InvalidPassword,
    #[error("Invalid organization name")]
    InvalidOrganizationName,
    #[error("Invalid token format")]
    InvalidToken,
    #[error("Invalid refresh token")]
    InvalidRefreshToken,

    // Authentication errors
    #[error("Authentication failed")]
    AuthenticationFailed,
    #[error("Token expired")]
    TokenExpired,
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("User not found")]
    UserNotFound,
    #[error("User already exists")]
    UserAlreadyExists,

    // Permission errors
    #[error("Insufficient permissions")]
    InsufficientPermissions,

    // Resource errors
    #[error("Organization not found")]
    OrganizationNotFound,
    #[error("Organization ID is required")]
    MissingOrganizationId,
    #[error("At least one role must be specified")]
    MissingRoles,

    // Request errors
    #[error("Missing request body")]
    MissingBody,
    #[error("Missing token")]
    MissingToken,

    // Operation errors
    #[error("Failed to create user: {0}")]
    UserCreationFailed(String),
    #[error("Failed to delete user: {0}")]
    UserDeletionFailed(String),
    #[error("Failed to update user: {0}")]
    UserUpdateFailed(String),
    #[error("Failed to retrieve users: {0}")]
    UserRetrievalFailed(String),
    #[error("Failed to refresh token: {0}")]
    TokenRefreshFailed(String),

    // Internal errors
    #[error("Internal server error: {0}")]
    InternalError(String),
}

impl LambdaError {
    /// Convert to HTTP status code
    pub fn status_code(&self) -> i64 {
        match self {
            // 400 Bad Request
            LambdaError::InvalidEmail
            | LambdaError::InvalidUsername
            | LambdaError::InvalidPassword
            | LambdaError::InvalidOrganizationName
            | LambdaError::InvalidToken
            | LambdaError::InvalidRefreshToken
            | LambdaError::MissingBody
            | LambdaError::MissingToken
            | LambdaError::MissingOrganizationId
            | LambdaError::MissingRoles => 400,

            // 401 Unauthorized
            LambdaError::AuthenticationFailed
            | LambdaError::TokenExpired
            | LambdaError::InvalidSignature => 401,

            // 403 Forbidden
            LambdaError::InsufficientPermissions => 403,

            // 404 Not Found
            LambdaError::UserNotFound | LambdaError::OrganizationNotFound => 404,

            // 409 Conflict
            LambdaError::UserAlreadyExists => 409,

            // 500 Internal Server Error
            LambdaError::UserCreationFailed(_)
            | LambdaError::UserDeletionFailed(_)
            | LambdaError::UserUpdateFailed(_)
            | LambdaError::UserRetrievalFailed(_)
            | LambdaError::TokenRefreshFailed(_)
            | LambdaError::InternalError(_) => 500,
        }
    }

    /// Get user-friendly error message
    pub fn user_message(&self) -> &'static str {
        match self {
            LambdaError::InvalidEmail => "Please provide a valid email address",
            LambdaError::InvalidUsername =>
                "Username must be 3-30 characters long and contain only letters, numbers, underscores, or hyphens",
            LambdaError::InvalidPassword =>
                "Password must be at least 8 characters long and contain uppercase, lowercase, and numbers",
            LambdaError::InvalidOrganizationName =>
                "Organization name must be between 2 and 100 characters",
            LambdaError::InvalidToken => "Invalid token provided",
            LambdaError::InvalidRefreshToken => "Invalid refresh token",
            LambdaError::AuthenticationFailed => "Invalid credentials",
            LambdaError::TokenExpired => "Token has expired",
            LambdaError::InvalidSignature => "Token signature verification failed",
            LambdaError::UserNotFound => "User not found",
            LambdaError::UserAlreadyExists => "A user with this email already exists",
            LambdaError::InsufficientPermissions =>
                "You don't have permission to perform this action",
            LambdaError::OrganizationNotFound => "Organization not found",
            LambdaError::MissingOrganizationId => "Organization ID is required",
            LambdaError::MissingRoles => "At least one role must be specified",
            LambdaError::MissingBody => "Request body is required",
            LambdaError::MissingToken => "Token is required",
            LambdaError::UserCreationFailed(_) => "Failed to create user. Please try again later",
            LambdaError::UserDeletionFailed(_) => "Failed to delete user. Please try again later",
            LambdaError::UserUpdateFailed(_) => "Failed to update user. Please try again later",
            LambdaError::UserRetrievalFailed(_) =>
                "Failed to retrieve user information. Please try again later",
            LambdaError::TokenRefreshFailed(_) => "Failed to refresh token. Please try again later",
            LambdaError::InternalError(_) => "An internal error occurred. Please try again later",
        }
    }
}

/// Result type for Lambda operations
pub type LambdaResult<T> = Result<T, LambdaError>;

/// Convert specific error types to LambdaError
pub trait ToLambdaError {
    fn to_lambda_error(self) -> LambdaError;
}

impl ToLambdaError for serde_json::Error {
    fn to_lambda_error(self) -> LambdaError {
        LambdaError::InternalError(format!("JSON parsing error: {}", self))
    }
}

impl ToLambdaError for std::io::Error {
    fn to_lambda_error(self) -> LambdaError {
        LambdaError::InternalError(format!("IO error: {}", self))
    }
}

impl ToLambdaError for anyhow::Error {
    fn to_lambda_error(self) -> LambdaError {
        LambdaError::InternalError(self.to_string())
    }
}
