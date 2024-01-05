use axum::{
    extract::rejection::{TypedHeaderRejection, TypedHeaderRejectionReason},
    response::{IntoResponse, Response},
    Json,
};
use headers::HeaderName;
use serde::Serialize;
use std::fmt::{Display, Formatter};

/// An error resulting from parsing request parameters
#[derive(Debug)]
pub struct Error {
    /// The name(s) of the header that caused the error
    pub names: Vec<&'static HeaderName>,
    /// Additional information about why the error occurred
    pub reason: Reason,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let names = {
            let length = self.names.iter().map(|n| n.as_str().len()).sum::<usize>();
            let mut acc = String::with_capacity(length);

            for name in &self.names {
                acc.push('`');
                acc.push_str(name.as_str());
                acc.push_str("`, ");
            }

            acc.pop();
            acc
        };

        match &self.reason {
            Reason::Missing => write!(f, "Header(s) of type {names} were missing"),
            Reason::Error(e) => write!(f, "{e} ({names})"),
        }
    }
}

impl From<TypedHeaderRejection> for Error {
    fn from(rejection: TypedHeaderRejection) -> Self {
        // SAFETY: this is safe to do since the `name` field of a `TypedHeaderRejection` lives for static as well
        let name =
            unsafe { std::mem::transmute::<&HeaderName, &'static HeaderName>(rejection.name()) };

        Self {
            names: vec![name],
            reason: Reason::Missing,
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (
            http::StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                message: self.to_string(),
            }),
        )
            .into_response()
    }
}

/// Additional information regarding an [`Error`]
#[derive(Debug)]
pub enum Reason {
    /// The header was missing from the HTTP request
    Missing,
    /// An error occurred when parsing the header
    Error(headers::Error),
}

impl From<TypedHeaderRejectionReason> for Reason {
    fn from(reason: TypedHeaderRejectionReason) -> Self {
        match reason {
            TypedHeaderRejectionReason::Missing => Reason::Missing,
            TypedHeaderRejectionReason::Error(e) => Reason::Error(e),
            _ => Reason::Missing,
        }
    }
}

/// The HTTP response for the error
#[derive(Serialize)]
struct ErrorResponse {
    message: String,
}
