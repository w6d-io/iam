use axum::{
    http::{header::ToStrError, StatusCode},
    response::{IntoResponse, Response},
};
use ory_kratos_client::apis::{metadata_api::IsReadyError, Error};
use thiserror::Error;

///handler for error in the http service
///it convert the recevied error in a response
#[derive(Error, Debug)]
pub enum RouterError {
    #[error("failed to apply identity patch")]
    Internal(#[from] anyhow::Error),
    #[error("failled to convert to string")]
    StrConvert(#[from] ToStrError),
    #[error("an error ocured when contacting kratos")]
    Kratos(#[from] Error<IsReadyError>),
}

impl IntoResponse for RouterError {
    fn into_response(self) -> Response {
        match self {
            RouterError::Internal(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Something went wrong: {}", e),
            )
                .into_response(),
            RouterError::StrConvert(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Something went wrong: {}", e),
            )
                .into_response(),
            RouterError::Kratos(e) => (
                StatusCode::SERVICE_UNAVAILABLE,
                format!("Kratos is not ready: {:?}", e),
            )
                .into_response(),
        }
    }
}
