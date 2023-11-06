/// Context information from the `events` service
pub mod event;
mod headers;
/// Context information from the `identity` service
pub mod user;

pub use headers::*;

#[cfg(test)]
mod test_util {
    #[macro_export]
    macro_rules! request {
        (
            $( $name:expr => $value:expr ),* $(,)?
        ) => {
            {
                let request = ::http::request::Request::builder()
                    $( .header($name, $value) )*
                    .body(())
                    .unwrap();
                let (parts, _) = request.into_parts();
                parts
            }
        };
    }

    #[macro_export]
    macro_rules! error_test_cases {
        ( $(
            $name:ident ( $( $header_name:expr => $header_value:expr ),* $(,)? ) => {
                header: $header:expr,
                reason: $reason:pat,
            }
        );+ $(;)? ) => {
            $(
                #[tokio::test]
                async fn $name() {
                    let mut parts = request! {
                        $( $header_name => $header_value, )*
                    };

                    let err = Context::from_request_parts(&mut parts, &()).await.unwrap_err();
                    assert_eq!(err.name().as_str(), $header);
                    assert!(matches!(*err.reason(), $reason));
                }
            )+
        };
    }
}
