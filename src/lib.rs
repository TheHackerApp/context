/// Pre-condition checks for use with [`async-graphql`](https://docs.rs/async-graphql)
#[cfg(feature = "graphql")]
pub mod checks;
#[cfg(feature = "headers")]
mod headers;
/// Context information from the `events` service
pub mod scope;
/// Context information from the `identity` service
pub mod user;

#[cfg(feature = "graphql")]
pub use checks::guard;
#[cfg(feature = "headers")]
pub use headers::*;

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
        ( $(
            $name:ident ( $( $header_name:expr => $header_value:expr ),* $(,)? ) => {
                header: $header:expr,
                kind: $kind:pat,
            }
        );+ $(;)? ) => {
            $(
                #[test]
                fn $name() {
                    let headers = $crate::headers! {
                        $( $header_name => $header_value, )*
                    };

                    let err = Context::try_from(&headers).unwrap_err();
                    assert_eq!(err.name.as_str(), $header);
                    assert!(matches!(err.kind, $kind));
                }
            )+
        };
    }
}
