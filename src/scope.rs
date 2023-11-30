#[cfg(feature = "extract")]
use crate::headers::{EventOrganizationId, EventSlug, RequestScope};
#[cfg(feature = "extract")]
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, HeaderMap},
    RequestPartsExt,
};
#[cfg(feature = "extract")]
use axum_extra::{
    headers::HeaderMapExt,
    typed_header::{TypedHeader, TypedHeaderRejection},
};
use serde::{
    de::{Error as _, MapAccess, Visitor},
    ser::SerializeMap,
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::{borrow::Cow, fmt::Formatter, marker::PhantomData};

/// Query parameters for fetching the scope
#[derive(Debug)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub enum Params<'p> {
    /// Find event context for a domain
    Domain(Cow<'p, str>),
    /// Find event context for a slug
    Slug(Cow<'p, str>),
}

impl<'de, 'p> Deserialize<'de> for Params<'p> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Default)]
        struct ParamsVisitor<'de, 'p> {
            marker: PhantomData<Params<'p>>,
            lifetime: PhantomData<&'de ()>,
        }

        impl<'de, 'p> Visitor<'de> for ParamsVisitor<'de, 'p> {
            type Value = Params<'p>;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                while let Some((key, value)) = map.next_entry::<&str, String>()? {
                    match key {
                        "slug" => return Ok(Params::Slug(Cow::Owned(value))),
                        "domain" => return Ok(Params::Domain(Cow::Owned(value))),
                        _ => continue,
                    }
                }

                Err(A::Error::custom("missing one of: `domain`, `slug`"))
            }
        }

        deserializer.deserialize_map(ParamsVisitor::default())
    }
}

impl<'p> Serialize for Params<'p> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;

        match self {
            Self::Domain(domain) => map.serialize_entry("domain", domain)?,
            Self::Slug(slug) => map.serialize_entry("slug", slug)?,
        };

        map.end()
    }
}

/// The scope response
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub enum Context {
    /// A request with global scope
    Admin,
    /// A request scoped to the current user (i.e. login, event selection)
    User,
    /// A request scoped to an event
    Event(EventContext),
}

#[cfg(feature = "extract")]
impl Context {
    /// Serialize the context into request headers
    pub fn into_headers(self) -> HeaderMap {
        let mut map = HeaderMap::with_capacity(1);
        self.write_headers(&mut map);
        map
    }

    /// Write the context to request headers
    pub fn write_headers(self, headers: &mut HeaderMap) {
        match self {
            Context::Admin => headers.typed_insert(RequestScope::Admin),
            Context::User => headers.typed_insert(RequestScope::User),
            Context::Event(context) => {
                headers.typed_insert(RequestScope::Event);
                context.write_headers(headers);
            }
        }
    }
}

#[cfg(feature = "extract")]
#[async_trait]
impl<S> FromRequestParts<S> for Context
where
    S: Send + Sync,
{
    type Rejection = TypedHeaderRejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let TypedHeader(scope) = parts.extract::<TypedHeader<RequestScope>>().await?;

        Ok(match scope {
            RequestScope::Admin => Self::Admin,
            RequestScope::User => Self::User,
            RequestScope::Event => {
                Self::Event(EventContext::from_request_parts(parts, state).await?)
            }
        })
    }
}

/// Additional information about an event
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub struct EventContext {
    /// The event slug
    pub event: String,
    /// The ID of the organization that manages the event
    pub organization_id: i32,
}

#[cfg(feature = "extract")]
impl EventContext {
    /// Write the context to request headers
    pub fn write_headers(self, headers: &mut HeaderMap) {
        headers.typed_insert(EventSlug::from(self.event));
        headers.typed_insert(EventOrganizationId::from(self.organization_id));
    }
}

#[cfg(feature = "extract")]
#[async_trait]
impl<S> FromRequestParts<S> for EventContext
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
    use super::Params;
    use std::borrow::Cow;

    #[test]
    fn round_trip_params_domain() {
        let params = Params::Domain(Cow::Borrowed("wafflehacks.org"));
        let encoded = serde_urlencoded::to_string(&params).unwrap();
        assert_eq!(encoded, "domain=wafflehacks.org");

        let decoded = serde_urlencoded::from_str(&encoded).unwrap();
        assert_eq!(params, decoded);
    }

    #[test]
    fn round_trip_params_slug() {
        let params = Params::Slug(Cow::Borrowed("wafflehacks-2023"));
        let encoded = serde_urlencoded::to_string(&params).unwrap();
        assert_eq!(encoded, "slug=wafflehacks-2023");

        let decoded = serde_urlencoded::from_str(&encoded).unwrap();
        assert_eq!(params, decoded);
    }
}

#[cfg(all(test, feature = "extract"))]
mod extract_tests {
    use super::{Context, EventContext, Params};
    use crate::{error_test_cases, request};
    use axum::{
        extract::{FromRequestParts, Query},
        http::Request,
    };
    use axum_extra::typed_header::TypedHeaderRejectionReason;
    use std::borrow::Cow;

    #[tokio::test]
    async fn params_domain_from_request() {
        let request = Request::builder()
            .uri("/context?domain=wafflehacks.org")
            .body(())
            .unwrap();
        let (mut parts, _) = request.into_parts();

        let Query(params) = Query::<Params>::from_request_parts(&mut parts, &())
            .await
            .unwrap();
        assert_eq!(params, Params::Domain(Cow::Borrowed("wafflehacks.org")));
    }

    #[tokio::test]
    async fn params_slug_from_request() {
        let request = Request::builder()
            .uri("/context?slug=wafflehacks-2023")
            .body(())
            .unwrap();
        let (mut parts, _) = request.into_parts();

        let Query(params) = Query::<Params>::from_request_parts(&mut parts, &())
            .await
            .unwrap();
        assert_eq!(params, Params::Slug(Cow::Borrowed("wafflehacks-2023")));
    }

    error_test_cases! {
        from_request_missing_scope() => {
            header: "request-scope",
            reason: TypedHeaderRejectionReason::Missing,
        };
        from_request_invalid_scope("Request-Scope" => "invalid") => {
            header: "request-scope",
            reason: TypedHeaderRejectionReason::Error(_),
        };
    }

    #[tokio::test]
    async fn from_request_admin_valid() {
        let mut parts = request! { "Request-Scope" => "admin" };

        let context = Context::from_request_parts(&mut parts, &()).await.unwrap();
        assert_eq!(context, Context::Admin);
    }

    #[tokio::test]
    async fn from_request_user_valid() {
        let mut parts = request! { "Request-Scope" => "user" };

        let context = Context::from_request_parts(&mut parts, &()).await.unwrap();
        assert_eq!(context, Context::User);
    }

    #[tokio::test]
    async fn from_request_event_valid() {
        let mut parts = request! {
            "Request-Scope" => "event",
            "Event-Slug" => "wafflehacks",
            "Event-Organization-ID" => "5",
        };

        let context = Context::from_request_parts(&mut parts, &()).await.unwrap();
        let Context::Event(context) = context else {
            panic!("expected Context::Event, got {context:?}")
        };

        assert_eq!(context.event, "wafflehacks");
        assert_eq!(context.organization_id, 5);
    }

    error_test_cases! {
        from_request_event_missing_slug(
            "Request-Scope" => "event",
            "Event-Organization-ID" => "5",
        ) => {
            header: "event-slug",
            reason: TypedHeaderRejectionReason::Missing,
        };
        from_request_event_slug_only_accepts_ascii(
            "Request-Scope" => "event",
            "Event-Slug" => "öne",
            "Event-Organization-ID" => "5",
        ) => {
            header: "event-slug",
            reason: TypedHeaderRejectionReason::Error(_),
        };
    }

    error_test_cases! {
        from_request_event_missing_organization_id(
            "Request-Scope" => "event",
            "event-slug" => "wafflehacks",
        ) => {
            header: "event-organization-id",
            reason: TypedHeaderRejectionReason::Missing,
        };
        from_request_event_invalid_organization_id(
            "Request-Scope" => "event",
            "Event-Slug" => "testing",
            "Event-Organization-ID" => "af",
        ) => {
            header: "event-organization-id",
            reason: TypedHeaderRejectionReason::Error(_),
        };
    }

    #[test]
    fn admin_into_headers() {
        let headers = Context::Admin.into_headers();
        assert_eq!(headers.get("request-scope").unwrap(), "admin");
    }

    #[test]
    fn user_into_headers() {
        let headers = Context::User.into_headers();
        assert_eq!(headers.get("request-scope").unwrap(), "user");
    }

    #[test]
    fn event_into_headers() {
        let context = Context::Event(EventContext {
            event: String::from("testing"),
            organization_id: 99,
        });
        let headers = context.into_headers();

        assert_eq!(headers.get("request-scope").unwrap(), "event");
        assert_eq!(headers.get("event-slug").unwrap(), "testing");
        assert_eq!(headers.get("event-organization-id").unwrap(), "99");
    }

    #[tokio::test]
    async fn round_trip_admin_context() {
        let context = Context::Admin;

        let mut request = Request::<()>::default();
        *request.headers_mut() = context.clone().into_headers();
        let (mut parts, _) = request.into_parts();

        let roundtripped = Context::from_request_parts(&mut parts, &()).await.unwrap();
        assert_eq!(context, roundtripped);
    }

    #[tokio::test]
    async fn round_trip_user_context() {
        let context = Context::User;

        let mut request = Request::<()>::default();
        *request.headers_mut() = context.clone().into_headers();
        let (mut parts, _) = request.into_parts();

        let roundtripped = Context::from_request_parts(&mut parts, &()).await.unwrap();
        assert_eq!(context, roundtripped);
    }

    #[tokio::test]
    async fn round_trip_event_context() {
        let context = Context::Event(EventContext {
            event: String::from("testing"),
            organization_id: 99,
        });

        let mut request = Request::<()>::default();
        *request.headers_mut() = context.clone().into_headers();
        let (mut parts, _) = request.into_parts();

        let roundtripped = Context::from_request_parts(&mut parts, &()).await.unwrap();
        assert_eq!(context, roundtripped);
    }
}
