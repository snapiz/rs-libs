use actix_web::http::StatusCode;
use async_graphql::{ErrorExtensions, FieldError};
use serde_json::json;
use validator::{ValidationErrors, ValidationErrorsKind};

#[derive(Debug, PartialEq, Error)]
pub enum Error {
    #[error("{0}")]
    BadRequest(String),

    #[error("Not Found")]
    NotFound,

    #[error("{0}")]
    Unauthorized(String),

    #[error("{0}")]
    Forbidden(String),

    #[error("{0}")]
    UnprocessableEntity(String),

    #[error("Internal Server Error")]
    InternalServerError,
}

impl From<ValidationErrors> for Error {
    fn from(e: ValidationErrors) -> Error {
        match e.errors().iter().next() {
            None => Error::InternalServerError,
            Some((field, kind)) => match kind {
                ValidationErrorsKind::Field(errors) => match errors.first() {
                    Some(e) => Error::UnprocessableEntity(format!(
                        "field: {}, code: {}, params: [{:?}]",
                        field, e.code, e.params
                    )),
                    None => Error::InternalServerError,
                },
                _ => Error::UnprocessableEntity(e.to_string()),
            },
        }
    }
}

impl ErrorExtensions for Error {
    fn extend(&self) -> FieldError {
        let status_code = match self {
            Error::BadRequest(_) => StatusCode::BAD_REQUEST,
            Error::NotFound => StatusCode::NOT_FOUND,
            Error::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Error::Forbidden(_) => StatusCode::FORBIDDEN,
            Error::UnprocessableEntity(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Error::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
        };

        FieldError(
            format!("{}", self),
            Some(json!({ "statusCode": status_code.as_u16() })),
        )
    }
}

pub type Result<T> = std::result::Result<T, Error>;
