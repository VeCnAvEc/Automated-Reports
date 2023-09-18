use actix_web::{error::ResponseError, http::StatusCode, HttpResponse};

use crate::r#trait::automated_report_response::Response;
use serde::Serialize;
use thiserror::Error;

#[derive(Serialize)]
pub struct ErrorResponse {
    pub code: u16,
    pub message: String,
    pub error: String,
}

#[derive(Error, Debug)]
pub enum CustomError {
    #[error("Requested file was not found")]
    NotFound,
    #[error("You are forbidden to access requested file.")]
    Forbidden,
    #[error("Not found file")]
    NotFoundFile,
    #[error("The authorization token is invalid")]
    Unauthorized,
    #[error("Unknown Internal Error")]
    Unknown,
}

impl CustomError {
    pub fn error_name(&self) -> String {
        match self {
            Self::NotFound => "NotFound".to_string(),
            Self::Forbidden => "Forbidden".to_string(),
            Self::NotFoundFile => "NotFoundFile".to_string(),
            Self::Unauthorized => "Unauthorized".to_string(),
            Self::Unknown => "Unknown".to_string(),
        }
    }
}

impl ResponseError for CustomError {
    fn status_code(&self) -> StatusCode {
        match *self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::NotFoundFile => StatusCode::NOT_FOUND,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::Unknown => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status_code = self.status_code();
        let error_response = Response::new::<String>(
            Some((
                4454391,
                self.to_string(),
            )),
            None,
            None
        );
        HttpResponse::build(status_code).json(error_response)
    }
}

pub fn map_io_error(e: std::io::Error) -> CustomError {
    match e.kind() {
        std::io::ErrorKind::NotFound => CustomError::NotFound,
        std::io::ErrorKind::PermissionDenied => CustomError::Forbidden,
        _ => CustomError::Unknown,
    }
}
