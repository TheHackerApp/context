#[cfg(feature = "headers")]
use crate::headers::{extract, EventOrganizationId, EventSlug, RequestScope};
#[cfg(feature = "axum")]
use axum_core::{
    extract::FromRequestParts,
    response::{IntoResponse, IntoResponseParts, Response, ResponseParts},
};
#[cfg(feature = "headers")]
use headers::HeaderMapExt;
#[cfg(feature = "axum")]
use http::request::Parts;
#[cfg(feature = "headers")]
use http::HeaderMap;
use serde::{
    de::{Error as _, MapAccess, Visitor},
    ser::SerializeMap,
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::{borrow::Cow, fmt::Formatter, marker::PhantomData};

/// Query parameters for fetching the scope
#[derive(Debug)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub enum ScopeParams<'p> {
    /// Find event context for a domain
    Domain(Cow<'p, str>),
    /// Find event context for a slug
    Slug(Cow<'p, str>),
}

impl<'de, 'p> Deserialize<'de> for ScopeParams<'p> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Default)]
        struct ParamsVisitor<'de, 'p> {
            marker: PhantomData<ScopeParams<'p>>,
            lifetime: PhantomData<&'de ()>,
        }

        impl<'de, 'p> Visitor<'de> for ParamsVisitor<'de, 'p> {
            type Value = ScopeParams<'p>;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                while let Some((key, value)) = map.next_entry::<&str, String>()? {
                    match key {
                        "slug" => return Ok(ScopeParams::Slug(Cow::Owned(value))),
                        "domain" => return Ok(ScopeParams::Domain(Cow::Owned(value))),
                        _ => continue,
                    }
                }

                Err(A::Error::custom("missing one of: `domain`, `slug`"))
            }
        }

        deserializer.deserialize_map(ParamsVisitor::default())
    }
}

impl<'p> Serialize for ScopeParams<'p> {
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

/// Information about the scope of the current request
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(test, derive(Eq, PartialEq))]
#[serde(rename_all = "lowercase", tag = "kind")]
pub enum Scope {
    /// A request with global scope
    Admin,
    /// A request scoped to the current user (i.e. login, event selection)
    User,
    /// A request scoped to an event
    Event(EventScope),
}

#[cfg(feature = "headers")]
impl Scope {
    /// Serialize the context into request headers
    pub fn into_headers(self) -> HeaderMap {
        let mut map = HeaderMap::with_capacity(1);
        self.write_headers(&mut map);
        map
    }

    /// Write the context to request headers
    pub fn write_headers(self, headers: &mut HeaderMap) {
        match self {
            Scope::Admin => headers.typed_insert(RequestScope::Admin),
            Scope::User => headers.typed_insert(RequestScope::User),
            Scope::Event(context) => {
                headers.typed_insert(RequestScope::Event);
                context.write_headers(headers);
            }
        }
    }
}

#[cfg(feature = "headers")]
impl TryFrom<&HeaderMap> for Scope {
    type Error = crate::Error;

    fn try_from(headers: &HeaderMap) -> Result<Self, Self::Error> {
        let scope = extract::<RequestScope>(headers)?;

        Ok(match scope {
            RequestScope::Admin => Self::Admin,
            RequestScope::User => Self::User,
            RequestScope::Event => {
                let context = EventScope::try_from(headers)?;
                Self::Event(context)
            }
        })
    }
}

#[cfg(feature = "axum")]
#[async_trait::async_trait]
impl<S> FromRequestParts<S> for Scope
where
    S: Send + Sync,
{
    type Rejection = crate::Error;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Self::try_from(&parts.headers)
    }
}

#[cfg(feature = "axum")]
impl IntoResponseParts for Scope {
    type Error = std::convert::Infallible;

    fn into_response_parts(self, mut res: ResponseParts) -> Result<ResponseParts, Self::Error> {
        self.write_headers(res.headers_mut());
        Ok(res)
    }
}

#[cfg(feature = "axum")]
impl IntoResponse for Scope {
    fn into_response(self) -> Response {
        self.into_headers().into_response()
    }
}

/// Additional information about a request scoped to an event
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub struct EventScope {
    /// The event slug
    pub event: String,
    /// The ID of the organization that manages the event
    pub organization_id: i32,
}

#[cfg(feature = "headers")]
impl EventScope {
    /// Write the context to request headers
    pub fn write_headers(self, headers: &mut HeaderMap) {
        headers.typed_insert(EventSlug::from(self.event));
        headers.typed_insert(EventOrganizationId::from(self.organization_id));
    }
}

#[cfg(feature = "headers")]
impl TryFrom<&HeaderMap> for EventScope {
    type Error = crate::Error;

    fn try_from(headers: &HeaderMap) -> Result<Self, Self::Error> {
        let event = extract::<EventSlug>(headers)?;
        let organization_id = extract::<EventOrganizationId>(headers)?;

        Ok(Self {
            event: event.into_inner(),
            organization_id: organization_id.into_inner(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{EventScope, Scope, ScopeParams};
    use std::borrow::Cow;

    #[test]
    fn round_trip_params_domain() {
        let params = ScopeParams::Domain(Cow::Borrowed("wafflehacks.org"));
        let encoded = serde_urlencoded::to_string(&params).unwrap();
        assert_eq!(encoded, "domain=wafflehacks.org");

        let decoded = serde_urlencoded::from_str(&encoded).unwrap();
        assert_eq!(params, decoded);
    }

    #[test]
    fn round_trip_params_slug() {
        let params = ScopeParams::Slug(Cow::Borrowed("wafflehacks-2023"));
        let encoded = serde_urlencoded::to_string(&params).unwrap();
        assert_eq!(encoded, "slug=wafflehacks-2023");

        let decoded = serde_urlencoded::from_str(&encoded).unwrap();
        assert_eq!(params, decoded);
    }

    #[test]
    fn context_admin_serializes_as_tagged_union() {
        let serialized = serde_json::to_string(&Scope::Admin).unwrap();
        assert_eq!(serialized, r#"{"kind":"admin"}"#);
    }

    #[test]
    fn context_user_serializes_as_tagged_union() {
        let serialized = serde_json::to_string(&Scope::User).unwrap();
        assert_eq!(serialized, r#"{"kind":"user"}"#);
    }

    #[test]
    fn context_event_serializes_as_tagged_union() {
        let ctx = Scope::Event(EventScope {
            event: String::from("testing"),
            organization_id: 45,
        });
        let serialized = serde_json::to_string(&ctx).unwrap();
        assert_eq!(
            serialized,
            r#"{"kind":"event","event":"testing","organization_id":45}"#
        );
    }
}

#[cfg(all(test, feature = "headers"))]
mod headers_tests {
    use super::{EventScope, Scope};
    use crate::{error_test_cases, headers, headers::ErrorKind};

    error_test_cases! {
        for Scope;
        try_from_missing_scope() => {
            header: "request-scope",
            kind: ErrorKind::Missing,
        };
        try_from_invalid_scope("Request-Scope" => "invalid") => {
            header: "request-scope",
            kind: ErrorKind::Error(_),
        };
    }

    #[tokio::test]
    async fn try_from_admin_valid() {
        let headers = headers! { "Request-Scope" => "admin" };

        let context = Scope::try_from(&headers).unwrap();
        assert_eq!(context, Scope::Admin);
    }

    #[tokio::test]
    async fn try_from_user_valid() {
        let headers = headers! { "Request-Scope" => "user" };

        let context = Scope::try_from(&headers).unwrap();
        assert_eq!(context, Scope::User);
    }

    #[tokio::test]
    async fn try_from_event_valid() {
        let headers = headers! {
            "Request-Scope" => "event",
            "Event-Slug" => "wafflehacks",
            "Event-Organization-ID" => "5",
        };

        let context = Scope::try_from(&headers).unwrap();
        let Scope::Event(context) = context else {
            panic!("expected Context::Event, got {context:?}")
        };

        assert_eq!(context.event, "wafflehacks");
        assert_eq!(context.organization_id, 5);
    }

    error_test_cases! {
        for Scope;
        try_from_event_missing_slug(
            "Request-Scope" => "event",
            "Event-Organization-ID" => "5",
        ) => {
            header: "event-slug",
            kind: ErrorKind::Missing,
        };
        try_from_event_slug_only_accepts_ascii(
            "Request-Scope" => "event",
            "Event-Slug" => "öne",
            "Event-Organization-ID" => "5",
        ) => {
            header: "event-slug",
            kind: ErrorKind::Error(_),
        };
    }

    error_test_cases! {
        for Scope;
        try_from_event_missing_organization_id(
            "Request-Scope" => "event",
            "event-slug" => "wafflehacks",
        ) => {
            header: "event-organization-id",
            kind: ErrorKind::Missing,
        };
        try_from_event_invalid_organization_id(
            "Request-Scope" => "event",
            "Event-Slug" => "testing",
            "Event-Organization-ID" => "af",
        ) => {
            header: "event-organization-id",
            kind: ErrorKind::Error(_),
        };
    }

    #[test]
    fn admin_into_headers() {
        let headers = Scope::Admin.into_headers();
        assert_eq!(headers.get("request-scope").unwrap(), "admin");
    }

    #[test]
    fn user_into_headers() {
        let headers = Scope::User.into_headers();
        assert_eq!(headers.get("request-scope").unwrap(), "user");
    }

    #[test]
    fn event_into_headers() {
        let context = Scope::Event(EventScope {
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
        let context = Scope::Admin;

        let headers = context.clone().into_headers();
        let roundtripped = Scope::try_from(&headers).unwrap();
        assert_eq!(context, roundtripped);
    }

    #[tokio::test]
    async fn round_trip_user_context() {
        let context = Scope::User;

        let headers = context.clone().into_headers();
        let roundtripped = Scope::try_from(&headers).unwrap();
        assert_eq!(context, roundtripped);
    }

    #[tokio::test]
    async fn round_trip_event_context() {
        let context = Scope::Event(EventScope {
            event: String::from("testing"),
            organization_id: 99,
        });

        let headers = context.clone().into_headers();
        let roundtripped = Scope::try_from(&headers).unwrap();
        assert_eq!(context, roundtripped);
    }
}

#[cfg(all(test, feature = "axum"))]
mod axum_tests {
    use super::ScopeParams;
    use axum::extract::{FromRequestParts, Query};
    use std::borrow::Cow;

    #[tokio::test]
    async fn params_domain_from_request() {
        let request = http::request::Request::builder()
            .uri("/context?domain=wafflehacks.org")
            .body(())
            .unwrap();
        let (mut parts, _) = request.into_parts();

        let Query(params) = Query::<ScopeParams>::from_request_parts(&mut parts, &())
            .await
            .unwrap();
        assert_eq!(
            params,
            ScopeParams::Domain(Cow::Borrowed("wafflehacks.org"))
        );
    }

    #[tokio::test]
    async fn params_slug_from_request() {
        let request = http::request::Request::builder()
            .uri("/context?slug=wafflehacks-2023")
            .body(())
            .unwrap();
        let (mut parts, _) = request.into_parts();

        let Query(params) = Query::<ScopeParams>::from_request_parts(&mut parts, &())
            .await
            .unwrap();
        assert_eq!(params, ScopeParams::Slug(Cow::Borrowed("wafflehacks-2023")));
    }
}
