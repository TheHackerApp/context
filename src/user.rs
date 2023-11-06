use crate::headers::{
    OAuthProviderSlug, OAuthUserEmail, OAuthUserId, UserEmail, UserFamilyName, UserGivenName,
    UserId, UserIsAdmin, UserSession,
};
use axum::{
    async_trait,
    extract::{rejection::TypedHeaderRejection, FromRequestParts, TypedHeader},
    RequestPartsExt,
};
use headers::HeaderMapExt;
use http::{request::Parts, HeaderMap};
use serde::{Deserialize, Serialize};

/// Query parameters for fetching the user context
#[derive(Debug, Deserialize)]
pub struct Params {
    /// The session token
    pub token: String,
}

/// The user context response
#[derive(Debug, Deserialize, Serialize)]
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
            Context::Unauthenticated => headers.typed_insert(UserSession::Unauthenticated),
            Context::OAuth => headers.typed_insert(UserSession::OAuth),
            Context::RegistrationNeeded(context) => {
                headers.typed_insert(UserSession::RegistrationNeeded);
                context.write_headers(headers);
            }
            Context::Authenticated(context) => {
                headers.typed_insert(UserSession::Authenticated);
                context.write_headers(headers);
            }
        }
    }
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
#[derive(Debug, Deserialize, Serialize)]
pub struct RegistrationNeededContext {
    /// The slug of the provider the user authenticated with
    pub provider: String,
    /// The user's ID according to the provider
    pub id: String,
    /// The user's primary email from the provider
    pub email: String,
}

impl RegistrationNeededContext {
    /// Write the context to request headers
    fn write_headers(self, headers: &mut HeaderMap) {
        headers.typed_insert(OAuthProviderSlug::from(self.provider));
        headers.typed_insert(OAuthUserId::from(self.id));
        headers.typed_insert(OAuthUserEmail::from(self.email));
    }
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
#[derive(Debug, Deserialize, Serialize)]
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

impl AuthenticatedContext {
    /// Write the context to request headers
    fn write_headers(self, headers: &mut HeaderMap) {
        headers.typed_insert(UserId::from(self.id));
        headers.typed_insert(UserGivenName::from(self.given_name));
        headers.typed_insert(UserFamilyName::from(self.family_name));
        headers.typed_insert(UserEmail::from(self.email));
        headers.typed_insert(UserIsAdmin::from(self.is_admin));
    }
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

#[cfg(test)]
mod tests {
    use super::Context;
    use crate::{error_test_cases, request};
    use axum::extract::{rejection::TypedHeaderRejectionReason, FromRequestParts};

    #[tokio::test]
    async fn valid_unauthenticated() {
        let mut parts = request! {
            "User-Session" => "unauthenticated",
        };

        let context = Context::from_request_parts(&mut parts, &()).await.unwrap();
        assert!(matches!(context, Context::Unauthenticated));
    }

    #[tokio::test]
    async fn valid_oauth() {
        let mut parts = request! {
            "User-Session" => "oauth",
        };

        let context = Context::from_request_parts(&mut parts, &()).await.unwrap();
        assert!(matches!(context, Context::OAuth));
    }

    #[tokio::test]
    async fn valid_registration_needed() {
        let mut parts = request! {
            "User-Session" => "registration-needed",
            "OAuth-Provider-Slug" => "google",
            "OAuth-User-ID" => "1234567890",
            "OAuth-User-Email" => "hello@world.com",
        };

        let context = Context::from_request_parts(&mut parts, &()).await.unwrap();
        let Context::RegistrationNeeded(context) = context else {
            panic!("expected Context::RegistrationNeeded, got {:?}", context);
        };

        assert_eq!(context.provider, "google");
        assert_eq!(context.id, "1234567890");
        assert_eq!(context.email, "hello@world.com");
    }

    #[tokio::test]
    async fn valid_authenticated() {
        let mut parts = request! {
            "User-Session" => "authenticated",
            "User-ID" => "55",
            "User-Given-Name" => "John",
            "User-Family-Name" => "Doe",
            "User-Email" => "john.doe@gmail.com",
            "User-Is-Admin" => "true",
        };

        let context = Context::from_request_parts(&mut parts, &()).await.unwrap();
        let Context::Authenticated(context) = context else {
            panic!("expected Context::Authenticated, got {:?}", context);
        };

        assert_eq!(context.id, 55);
        assert_eq!(context.given_name, "John");
        assert_eq!(context.family_name, "Doe");
        assert_eq!(context.email, "john.doe@gmail.com");
        assert!(context.is_admin);
    }

    error_test_cases! {
        missing_session_state() => {
            header: "user-session",
            reason: TypedHeaderRejectionReason::Missing,
        };
        invalid_session_state("User-Session" => "unknown") => {
            header: "user-session",
            reason: TypedHeaderRejectionReason::Error(_),
        };
    }

    error_test_cases! {
        registration_needed_missing_oauth_provider(
            "User-Session" => "registration-needed",
            "OAuth-User-ID" => "1234567890",
            "OAuth-User-Email" => "hello@world.com",
        ) => {
            header: "oauth-provider-slug",
            reason: TypedHeaderRejectionReason::Missing,
        };
        registration_needed_oauth_provider_only_accepts_ascii(
            "User-Session" => "registration-needed",
            "OAuth-Provider-Slug" => "göögle",
            "OAuth-User-ID" => "1234567890",
            "OAuth-User-Email" => "hello@world.com",
        ) => {
            header: "oauth-provider-slug",
            reason: TypedHeaderRejectionReason::Error(_),
        };
        registration_needed_missing_user_id(
            "User-Session" => "registration-needed",
            "OAuth-Provider-Slug" => "google",
            "OAuth-User-Email" => "hello@world.com",
        ) => {
            header: "oauth-user-id",
            reason: TypedHeaderRejectionReason::Missing,
        };
        registration_needed_user_id_only_accepts_ascii(
            "User-Session" => "registration-needed",
            "OAuth-Provider-Slug" => "google",
            "OAuth-User-ID" => "123456789ö",
            "OAuth-User-Email" => "hello@world.com",
        ) => {
            header: "oauth-user-id",
            reason: TypedHeaderRejectionReason::Error(_),
        };
        registration_needed_missing_user_email(
            "User-Session" => "registration-needed",
            "OAuth-Provider-Slug" => "google",
            "OAuth-User-ID" => "1234567890",
        ) => {
            header: "oauth-user-email",
            reason: TypedHeaderRejectionReason::Missing,
        };
    }

    #[tokio::test]
    async fn registration_needed_user_email_accepts_utf8() {
        let mut parts = request! {
            "User-Session" => "registration-needed",
            "OAuth-Provider-Slug" => "google",
            "OAuth-User-ID" => "1234567890",
            "OAuth-User-Email" => "hellö@wörld.cöm",
        };

        let context = Context::from_request_parts(&mut parts, &()).await.unwrap();
        let Context::RegistrationNeeded(context) = context else {
            panic!("expected Context::RegistrationNeeded, got {:?}", context);
        };
        assert_eq!(context.email, "hellö@wörld.cöm");
    }

    error_test_cases! {
        authenticated_missing_id(
            "User-Session" => "authenticated",
            "User-Given-Name" => "John",
            "User-Family-Name" => "Doe",
            "User-Email" => "john.doe@gmail.com",
            "User-Is-Admin" => "true",
        ) => {
            header: "user-id",
            reason: TypedHeaderRejectionReason::Missing,
        };
        authenticated_invalid_id(
            "User-Session" => "authenticated",
            "User-ID" => "af",
            "User-Given-Name" => "John",
            "User-Family-Name" => "Doe",
            "User-Email" => "john.doe@gmail.com",
            "User-Is-Admin" => "true",
        ) => {
            header: "user-id",
            reason: TypedHeaderRejectionReason::Error(_),
        };
        authenticated_missing_given_name(
            "User-Session" => "authenticated",
            "User-ID" => "55",
            "User-Family-Name" => "Doe",
            "User-Email" => "john.doe@gmail.com",
            "User-Is-Admin" => "true",
        ) => {
            header: "user-given-name",
            reason: TypedHeaderRejectionReason::Missing,
        };
        authenticated_missing_family_name(
            "User-Session" => "authenticated",
            "User-ID" => "55",
            "User-Given-Name" => "John",
            "User-Email" => "john.doe@gmail.com",
            "User-Is-Admin" => "true",
        ) => {
            header: "user-family-name",
            reason: TypedHeaderRejectionReason::Missing,
        };
        authenticated_missing_email(
            "User-Session" => "authenticated",
            "User-ID" => "55",
            "User-Given-Name" => "John",
            "User-Family-Name" => "Doe",
            "User-Is-Admin" => "true",
        ) => {
            header: "user-email",
            reason: TypedHeaderRejectionReason::Missing,
        };
        authenticated_missing_is_admin(
            "User-Session" => "authenticated",
            "User-ID" => "55",
            "User-Given-Name" => "John",
            "User-Family-Name" => "Doe",
            "User-Email" => "john.doe@gmail.com",
        ) => {
            header: "user-is-admin",
            reason: TypedHeaderRejectionReason::Missing,
        };
        authenticated_invalid_is_admin(
            "User-Session" => "authenticated",
            "User-ID" => "55",
            "User-Given-Name" => "John",
            "User-Family-Name" => "Doe",
            "User-Email" => "john.doe@gmail.com",
            "User-Is-Admin" => "0",
        ) => {
            header: "user-is-admin",
            reason: TypedHeaderRejectionReason::Error(_),
        }
    }

    #[tokio::test]
    async fn authenticated_given_name_accepts_utf8() {
        let mut parts = request! {
            "User-Session" => "authenticated",
            "User-ID" => "55",
            "User-Given-Name" => "Jöhn",
            "User-Family-Name" => "Doe",
            "User-Email" => "john.doe@gmail.com",
            "User-Is-Admin" => "true",
        };

        let context = Context::from_request_parts(&mut parts, &()).await.unwrap();
        let Context::Authenticated(context) = context else {
            panic!("expected Context::Authenticated, got {:?}", context);
        };
        assert_eq!(context.given_name, "Jöhn");
    }

    #[tokio::test]
    async fn authenticated_family_name_accepts_utf8() {
        let mut parts = request! {
            "User-Session" => "authenticated",
            "User-ID" => "55",
            "User-Given-Name" => "John",
            "User-Family-Name" => "Döe",
            "User-Email" => "john.doe@gmail.com",
            "User-Is-Admin" => "true",
        };

        let context = Context::from_request_parts(&mut parts, &()).await.unwrap();
        let Context::Authenticated(context) = context else {
            panic!("expected Context::Authenticated, got {:?}", context);
        };
        assert_eq!(context.family_name, "Döe");
    }

    #[tokio::test]
    async fn authenticated_email_accepts_utf8() {
        let mut parts = request! {
            "User-Session" => "authenticated",
            "User-ID" => "55",
            "User-Given-Name" => "John",
            "User-Family-Name" => "Doe",
            "User-Email" => "jöhn.döe@gmail.cöm",
            "User-Is-Admin" => "true",
        };

        let context = Context::from_request_parts(&mut parts, &()).await.unwrap();
        let Context::Authenticated(context) = context else {
            panic!("expected Context::Authenticated, got {:?}", context);
        };
        assert_eq!(context.email, "jöhn.döe@gmail.cöm");
    }
}
