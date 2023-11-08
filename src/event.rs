use crate::headers::{EventOrganizationId, EventSlug};
use axum::{
    async_trait,
    extract::{rejection::TypedHeaderRejection, FromRequestParts, TypedHeader},
    RequestPartsExt,
};
use headers::HeaderMapExt;
use http::{request::Parts, HeaderMap};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// Query parameters for fetching the event context
#[derive(Debug, Deserialize, Serialize)]
pub struct Params<'p> {
    /// The domain to find the event context for
    pub domain: Cow<'p, str>,
}

/// The event context response
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub struct Context {
    /// The event slug
    pub event: String,
    /// The ID of the organization that manages the event
    pub organization_id: i32,
}

impl Context {
    /// Serialize the context into request headers
    pub fn into_headers(self) -> HeaderMap {
        let mut map = HeaderMap::with_capacity(2);
        self.write_headers(&mut map);
        map
    }

    /// Write the context to request headers
    pub fn write_headers(self, headers: &mut HeaderMap) {
        headers.typed_insert(EventSlug::from(self.event));
        headers.typed_insert(EventOrganizationId::from(self.organization_id));
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for Context
where
    S: Send + Sync,
{
    type Rejection = TypedHeaderRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let TypedHeader(event) = parts.extract::<TypedHeader<EventSlug>>().await?;
        let TypedHeader(organization_id) =
            parts.extract::<TypedHeader<EventOrganizationId>>().await?;

        Ok(Self {
            event: event.into_inner(),
            organization_id: organization_id.into_inner(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Context;
    use crate::{error_test_cases, request};
    use axum::extract::{rejection::TypedHeaderRejectionReason, FromRequestParts};
    use http::Request;

    #[tokio::test]
    async fn from_request_valid() {
        let mut parts = request! {
            "Event-Slug" => "wafflehacks",
            "Event-Organization-ID" => "5",
        };

        let context = Context::from_request_parts(&mut parts, &()).await.unwrap();
        assert_eq!(context.event, "wafflehacks");
        assert_eq!(context.organization_id, 5);
    }

    error_test_cases! {
        from_request_missing_slug("Event-Organization-ID" => "5") => {
            header: "event-slug",
            reason: TypedHeaderRejectionReason::Missing,
        };
        from_request_slug_only_accepts_ascii("Event-Slug" => "öne", "Event-Organization-ID" => "5") => {
            header: "event-slug",
            reason: TypedHeaderRejectionReason::Error(_),
        };
    }

    error_test_cases! {
        from_request_missing_organization_id("event-slug" => "wafflehacks") => {
            header: "event-organization-id",
            reason: TypedHeaderRejectionReason::Missing,
        };
        from_request_invalid_organization_id("Event-Slug" => "testing", "Event-Organization-ID" => "af") => {
            header: "event-organization-id",
            reason: TypedHeaderRejectionReason::Error(_),
        };
    }

    #[test]
    fn into_headers() {
        let context = Context {
            event: String::from("wafflehacks"),
            organization_id: 6,
        };
        let headers = context.into_headers();

        assert_eq!(headers.get("event-slug").unwrap(), "wafflehacks");
        assert_eq!(headers.get("event-organization-id").unwrap(), "6");
    }

    #[tokio::test]
    async fn roundtrip() {
        let context = Context {
            event: String::from("testing"),
            organization_id: 99,
        };

        let mut request = Request::<()>::default();
        *request.headers_mut() = context.clone().into_headers();
        let (mut parts, _) = request.into_parts();

        let roundtripped = Context::from_request_parts(&mut parts, &()).await.unwrap();
        assert_eq!(context, roundtripped);
    }
}
