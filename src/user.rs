use crate::headers::{
    OAuthProviderSlug, OAuthUserEmail, OAuthUserId, UserEmail, UserFamilyName, UserGivenName,
    UserId, UserIsAdmin, UserSession,
};
use axum::{
    async_trait,
    extract::{rejection::TypedHeaderRejection, FromRequestParts, TypedHeader},
    RequestPartsExt,
};
use http::request::Parts;
use serde::{Deserialize, Serialize};

/// Query parameters for fetching the user context
#[derive(Debug, Deserialize)]
pub struct Params {
    /// The session token
    pub token: String,
}

/// The user context response
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Context {
    /// The user is unauthenticated
    Unauthenticated,
    /// The user is in the middle of logging in via OAuth
    #[serde(rename = "oauth")]
    OAuth,
    /// The user needs to complete their registration
    RegistrationNeeded(RegistrationNeededContext),
    /// The user is fully authenticated
    Authenticated(AuthenticatedContext),
}

#[async_trait]
impl<S> FromRequestParts<S> for Context
where
    S: Send + Sync,
{
    type Rejection = TypedHeaderRejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let TypedHeader(session) = parts.extract::<TypedHeader<UserSession>>().await?;

        Ok(match session {
            UserSession::Unauthenticated => Self::Unauthenticated,
            UserSession::OAuth => Self::OAuth,
            UserSession::RegistrationNeeded => Self::RegistrationNeeded(
                RegistrationNeededContext::from_request_parts(parts, state).await?,
            ),
            UserSession::Authenticated => {
                Self::Authenticated(AuthenticatedContext::from_request_parts(parts, state).await?)
            }
        })
    }
}

/// Context parameters when the user needs to finish their registration
#[derive(Debug, Serialize)]
pub struct RegistrationNeededContext {
    /// The slug of the provider the user authenticated with
    pub provider: String,
    /// The user's ID according to the provider
    pub id: String,
    /// The user's primary email from the provider
    pub email: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for RegistrationNeededContext
where
    S: Send + Sync,
{
    type Rejection = TypedHeaderRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let TypedHeader(provider) = parts.extract::<TypedHeader<OAuthProviderSlug>>().await?;
        let TypedHeader(id) = parts.extract::<TypedHeader<OAuthUserId>>().await?;
        let TypedHeader(email) = parts.extract::<TypedHeader<OAuthUserEmail>>().await?;

        Ok(Self {
            provider: provider.into_inner(),
            id: id.into_inner(),
            email: email.into_inner(),
        })
    }
}

/// Context parameters when the user is authenticated
#[derive(Debug, Serialize)]
pub struct AuthenticatedContext {
    /// The user's ID
    pub id: i32,
    /// The user's given/first name
    pub given_name: String,
    /// The user's family/last name
    pub family_name: String,
    /// The user's primary email
    pub email: String,
    /// Whether the user is an admin
    pub is_admin: bool,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthenticatedContext
where
    S: Send + Sync,
{
    type Rejection = TypedHeaderRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let TypedHeader(id) = parts.extract::<TypedHeader<UserId>>().await?;
        let TypedHeader(given_name) = parts.extract::<TypedHeader<UserGivenName>>().await?;
        let TypedHeader(family_name) = parts.extract::<TypedHeader<UserFamilyName>>().await?;
        let TypedHeader(email) = parts.extract::<TypedHeader<UserEmail>>().await?;
        let TypedHeader(is_admin) = parts.extract::<TypedHeader<UserIsAdmin>>().await?;

        Ok(Self {
            id: id.into_inner(),
            given_name: given_name.into_inner(),
            family_name: family_name.into_inner(),
            email: email.into_inner(),
            is_admin: is_admin.into_inner(),
        })
    }
}
