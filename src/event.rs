use crate::headers::{EventOrganizationId, EventSlug};
use axum::{
    async_trait,
    extract::{rejection::TypedHeaderRejection, FromRequestParts, TypedHeader},
    RequestPartsExt,
};
use http::request::Parts;
use serde::{Deserialize, Serialize};

/// Query parameters for fetching the event context
#[derive(Debug, Deserialize)]
pub struct Params {
    /// The domain to find the event context for
    pub domain: String,
}

/// The event context response
#[derive(Debug, Serialize)]
pub struct Context {
    /// The event slug
    pub event: String,
    /// The ID of the organization that manages the event
    pub organization_id: i32,
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
