//! Authentication and authorization context that is passed between The Hacker App services.
//!
//! The two primary structs that are used to fetch and retrieve information are [`Scope`] and [`User`]. The [`Scope`]
//! contains information about how the request is being made (i.e. where is it from, is it for a particular event).
//! Whereas the [`User`] contains information about who is making the request.

#[cfg(feature = "graphql")]
pub mod checks;
#[cfg(feature = "headers")]
pub mod headers;

mod scope;
mod user;

#[cfg(feature = "graphql")]
pub use checks::guard;
#[cfg(feature = "headers")]
pub use headers::Error;
pub use scope::{EventScope, Scope, ScopeParams};
pub use user::{AuthenticatedUser, User, UserParams, UserRegistrationNeeded, UserRole};

#[cfg(test)]
mod test_util {
    #[macro_export]
    macro_rules! headers {
        () => {
            ::http::header::HeaderMap::with_capacity(0)
        };
        (
            $( $name:expr => $value:expr ),* $(,)?
        ) => {{
            let mut headers = ::http::header::HeaderMap::new();
            $(headers.insert($name, ::http::header::HeaderValue::try_from($value).unwrap());)*
            headers
        }};
    }

    #[macro_export]
    macro_rules! error_test_cases {
        (
            for $ctx:ident;
            $( $name:ident ( $( $header_name:expr => $header_value:expr ),* $(,)? ) => {
                header: $header:expr,
                kind: $kind:pat,
            } );+ $(;)?
        ) => {
            $(
                #[test]
                fn $name() {
                    let headers = $crate::headers! {
                        $( $header_name => $header_value, )*
                    };

                    let err = $ctx::try_from(&headers).unwrap_err();
                    assert_eq!(err.name.as_str(), $header);
                    assert!(matches!(err.kind, $kind));
                }
            )+
        };
    }
}
