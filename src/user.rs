#[cfg(feature = "headers")]
use crate::headers::{
    extract, extract_opt, OAuthProviderSlug, OAuthUserEmail, OAuthUserId, UserEmail,
    UserFamilyName, UserGivenName, UserId, UserIsAdmin, UserSession,
};
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
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// Query parameters for fetching the user context
#[derive(Debug, Deserialize, Serialize)]
pub struct UserParams<'p> {
    /// The session token
    #[serde(default)]
    pub token: Cow<'p, str>,
}

/// Information about the requesting user
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(test, derive(Eq, PartialEq))]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum User {
    /// The user is unauthenticated
    Unauthenticated,
    /// The user is in the middle of logging in via OAuth
    #[serde(rename = "oauth")]
    OAuth,
    /// The user needs to complete their registration
    RegistrationNeeded(UserRegistrationNeeded),
    /// The user is fully authenticated
    Authenticated(AuthenticatedUser),
}

#[cfg(feature = "headers")]
impl User {
    /// Serialize the context into request headers
    pub fn into_headers(self) -> HeaderMap {
        let mut map = HeaderMap::with_capacity(1);
        self.write_headers(&mut map);
        map
    }

    /// Write the context to request headers
    pub fn write_headers(self, headers: &mut HeaderMap) {
        match self {
            User::Unauthenticated => headers.typed_insert(UserSession::Unauthenticated),
            User::OAuth => headers.typed_insert(UserSession::OAuth),
            User::RegistrationNeeded(context) => {
                headers.typed_insert(UserSession::RegistrationNeeded);
                context.write_headers(headers);
            }
            User::Authenticated(context) => {
                headers.typed_insert(UserSession::Authenticated);
                context.write_headers(headers);
            }
        }
    }
}

#[cfg(feature = "headers")]
impl TryFrom<&HeaderMap> for User {
    type Error = crate::Error;

    fn try_from(headers: &HeaderMap) -> Result<Self, Self::Error> {
        let session = extract::<UserSession>(headers)?;

        Ok(match session {
            UserSession::Unauthenticated => Self::Unauthenticated,
            UserSession::OAuth => Self::OAuth,
            UserSession::RegistrationNeeded => {
                let context = UserRegistrationNeeded::try_from(headers)?;
                Self::RegistrationNeeded(context)
            }
            UserSession::Authenticated => {
                let context = AuthenticatedUser::try_from(headers)?;
                Self::Authenticated(context)
            }
        })
    }
}

#[cfg(feature = "axum")]
#[async_trait::async_trait]
impl<S> FromRequestParts<S> for User
where
    S: Send + Sync,
{
    type Rejection = crate::Error;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Self::try_from(&parts.headers)
    }
}

#[cfg(feature = "axum")]
impl IntoResponseParts for User {
    type Error = std::convert::Infallible;

    fn into_response_parts(self, mut res: ResponseParts) -> Result<ResponseParts, Self::Error> {
        self.write_headers(res.headers_mut());
        Ok(res)
    }
}

#[cfg(feature = "axum")]
impl IntoResponse for User {
    fn into_response(self) -> Response {
        self.into_headers().into_response()
    }
}

/// Details about a user that needs to complete their registration
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub struct UserRegistrationNeeded {
    /// The slug of the provider the user authenticated with
    pub provider: String,
    /// The user's ID according to the provider
    pub id: String,
    /// The user's primary email from the provider
    pub email: String,
}

#[cfg(feature = "headers")]
impl UserRegistrationNeeded {
    /// Write the context to request headers
    fn write_headers(self, headers: &mut HeaderMap) {
        headers.typed_insert(OAuthProviderSlug::from(self.provider));
        headers.typed_insert(OAuthUserId::from(self.id));
        headers.typed_insert(OAuthUserEmail::from(self.email));
    }
}

#[cfg(feature = "headers")]
impl TryFrom<&HeaderMap> for UserRegistrationNeeded {
    type Error = crate::Error;

    fn try_from(headers: &HeaderMap) -> Result<Self, Self::Error> {
        let provider = extract::<OAuthProviderSlug>(headers)?;
        let id = extract::<OAuthUserId>(headers)?;
        let email = extract::<OAuthUserEmail>(headers)?;

        Ok(Self {
            provider: provider.into_inner(),
            id: id.into_inner(),
            email: email.into_inner(),
        })
    }
}

/// Details about an authenticated user
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub struct AuthenticatedUser {
    /// The user's ID
    pub id: i32,
    /// The user's given/first name
    pub given_name: String,
    /// The user's family/last name
    pub family_name: String,
    /// The user's primary email
    pub email: String,
    /// The user's role for the scope
    pub role: Option<UserRole>,
    /// Whether the user is an admin
    pub is_admin: bool,
}

#[cfg(feature = "headers")]
impl AuthenticatedUser {
    /// Write the context to request headers
    fn write_headers(self, headers: &mut HeaderMap) {
        headers.typed_insert(UserId::from(self.id));
        headers.typed_insert(UserGivenName::from(self.given_name));
        headers.typed_insert(UserFamilyName::from(self.family_name));
        headers.typed_insert(UserEmail::from(self.email));
        if let Some(role) = self.role {
            headers.typed_insert(role);
        }
        headers.typed_insert(UserIsAdmin::from(self.is_admin));
    }
}

#[cfg(feature = "headers")]
impl TryFrom<&HeaderMap> for AuthenticatedUser {
    type Error = crate::Error;

    fn try_from(headers: &HeaderMap) -> Result<Self, Self::Error> {
        let id = extract::<UserId>(headers)?;
        let given_name = extract::<UserGivenName>(headers)?;
        let family_name = extract::<UserFamilyName>(headers)?;
        let email = extract::<UserEmail>(headers)?;
        let role = extract_opt::<UserRole>(headers)?;
        let is_admin = extract::<UserIsAdmin>(headers)?;

        Ok(Self {
            id: id.into_inner(),
            given_name: given_name.into_inner(),
            family_name: family_name.into_inner(),
            email: email.into_inner(),
            role,
            is_admin: is_admin.into_inner(),
        })
    }
}

/// The role applied to the current user
///
/// Transmitted in the `User-Role` header
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum UserRole {
    /// A participant of an event
    ///
    /// Cannot affect anything at the organization level, only has permissions for the individual
    /// event.
    Participant,
    /// A normal user within the organization
    Organizer,
    /// An elevated user within the organization
    ///
    /// Has more permissions than an organizer but less than a director. Managers are able to
    /// event and organization settings.
    Manager,
    /// The highest permissions in an organization
    ///
    /// Equivalent to an owner, but cannot modify billing information or delete the organization.
    Director,
}

#[cfg(all(test, feature = "headers"))]
mod tests {
    use super::{AuthenticatedUser, User, UserRegistrationNeeded, UserRole};
    use crate::{error_test_cases, headers, headers::ErrorKind};

    #[test]
    fn try_from_valid_unauthenticated() {
        let headers = headers! {
            "User-Session" => "unauthenticated",
        };

        let context = User::try_from(&headers).unwrap();
        assert!(matches!(context, User::Unauthenticated));
    }

    #[test]
    fn try_from_valid_oauth() {
        let headers = headers! {
            "User-Session" => "oauth",
        };

        let context = User::try_from(&headers).unwrap();
        assert!(matches!(context, User::OAuth));
    }

    #[test]
    fn try_from_valid_registration_needed() {
        let headers = headers! {
            "User-Session" => "registration-needed",
            "OAuth-Provider-Slug" => "google",
            "OAuth-User-ID" => "1234567890",
            "OAuth-User-Email" => "hello@world.com",
        };

        let context = User::try_from(&headers).unwrap();
        let User::RegistrationNeeded(context) = context else {
            panic!("expected Context::RegistrationNeeded, got {:?}", context);
        };

        assert_eq!(context.provider, "google");
        assert_eq!(context.id, "1234567890");
        assert_eq!(context.email, "hello@world.com");
    }

    #[test]
    fn try_from_valid_authenticated() {
        let headers = headers! {
            "User-Session" => "authenticated",
            "User-ID" => "55",
            "User-Given-Name" => "John",
            "User-Family-Name" => "Doe",
            "User-Email" => "john.doe@gmail.com",
            "User-Role" => "manager",
            "User-Is-Admin" => "true",
        };

        let context = User::try_from(&headers).unwrap();
        let User::Authenticated(context) = context else {
            panic!("expected Context::Authenticated, got {:?}", context);
        };

        assert_eq!(context.id, 55);
        assert_eq!(context.given_name, "John");
        assert_eq!(context.family_name, "Doe");
        assert_eq!(context.email, "john.doe@gmail.com");
        assert!(context.is_admin);
    }

    error_test_cases! {
        for User;
        try_from_missing_session_state() => {
            header: "user-session",
            kind: ErrorKind::Missing,
        };
        try_from_invalid_session_state("User-Session" => "unknown") => {
            header: "user-session",
            kind: ErrorKind::Error(_),
        };
    }

    error_test_cases! {
        for User;
        try_from_registration_needed_missing_oauth_provider(
            "User-Session" => "registration-needed",
            "OAuth-User-ID" => "1234567890",
            "OAuth-User-Email" => "hello@world.com",
        ) => {
            header: "oauth-provider-slug",
            kind: ErrorKind::Missing,
        };
        try_from_registration_needed_oauth_provider_only_accepts_ascii(
            "User-Session" => "registration-needed",
            "OAuth-Provider-Slug" => "göögle",
            "OAuth-User-ID" => "1234567890",
            "OAuth-User-Email" => "hello@world.com",
        ) => {
            header: "oauth-provider-slug",
            kind: ErrorKind::Error(_),
        };
        try_from_registration_needed_missing_user_id(
            "User-Session" => "registration-needed",
            "OAuth-Provider-Slug" => "google",
            "OAuth-User-Email" => "hello@world.com",
        ) => {
            header: "oauth-user-id",
            kind: ErrorKind::Missing,
        };
        try_from_registration_needed_user_id_only_accepts_ascii(
            "User-Session" => "registration-needed",
            "OAuth-Provider-Slug" => "google",
            "OAuth-User-ID" => "123456789ö",
            "OAuth-User-Email" => "hello@world.com",
        ) => {
            header: "oauth-user-id",
            kind: ErrorKind::Error(_),
        };
        try_from_registration_needed_missing_user_email(
            "User-Session" => "registration-needed",
            "OAuth-Provider-Slug" => "google",
            "OAuth-User-ID" => "1234567890",
        ) => {
            header: "oauth-user-email",
            kind: ErrorKind::Missing,
        };
    }

    #[test]
    fn try_from_registration_needed_user_email_accepts_utf8() {
        let headers = headers! {
            "User-Session" => "registration-needed",
            "OAuth-Provider-Slug" => "google",
            "OAuth-User-ID" => "1234567890",
            "OAuth-User-Email" => "hellö@wörld.cöm",
        };

        let context = User::try_from(&headers).unwrap();
        let User::RegistrationNeeded(context) = context else {
            panic!("expected Context::RegistrationNeeded, got {:?}", context);
        };
        assert_eq!(context.email, "hellö@wörld.cöm");
    }

    error_test_cases! {
        for User;
        try_from_authenticated_missing_id(
            "User-Session" => "authenticated",
            "User-Given-Name" => "John",
            "User-Family-Name" => "Doe",
            "User-Email" => "john.doe@gmail.com",
            "User-Role" => "organizer",
            "User-Is-Admin" => "true",
        ) => {
            header: "user-id",
            kind: ErrorKind::Missing,
        };
        try_from_authenticated_invalid_id(
            "User-Session" => "authenticated",
            "User-ID" => "af",
            "User-Given-Name" => "John",
            "User-Family-Name" => "Doe",
            "User-Email" => "john.doe@gmail.com",
            "User-Role" => "organizer",
            "User-Is-Admin" => "true",
        ) => {
            header: "user-id",
            kind: ErrorKind::Error(_),
        };
        try_from_authenticated_missing_given_name(
            "User-Session" => "authenticated",
            "User-ID" => "55",
            "User-Family-Name" => "Doe",
            "User-Email" => "john.doe@gmail.com",
            "User-Role" => "organizer",
            "User-Is-Admin" => "true",
        ) => {
            header: "user-given-name",
            kind: ErrorKind::Missing,
        };
        try_from_authenticated_missing_family_name(
            "User-Session" => "authenticated",
            "User-ID" => "55",
            "User-Given-Name" => "John",
            "User-Email" => "john.doe@gmail.com",
            "User-Role" => "organizer",
            "User-Is-Admin" => "true",
        ) => {
            header: "user-family-name",
            kind: ErrorKind::Missing,
        };
        try_from_authenticated_missing_email(
            "User-Session" => "authenticated",
            "User-ID" => "55",
            "User-Given-Name" => "John",
            "User-Family-Name" => "Doe",
            "User-Role" => "organizer",
            "User-Is-Admin" => "true",
        ) => {
            header: "user-email",
            kind: ErrorKind::Missing,
        };
        try_from_authenticated_invalid_role(
            "User-Session" => "authenticated",
            "User-ID" => "55",
            "User-Given-Name" => "John",
            "User-Family-Name" => "Doe",
            "User-Email" => "john.doe@gmail.com",
            "User-Role" => "developer",
            "User-Is-Admin" => "true",
        ) => {
            header: "user-role",
            kind: ErrorKind::Error(_),
        };
        try_from_authenticated_missing_is_admin(
            "User-Session" => "authenticated",
            "User-ID" => "55",
            "User-Given-Name" => "John",
            "User-Family-Name" => "Doe",
            "User-Email" => "john.doe@gmail.com",
            "User-Role" => "organizer",
        ) => {
            header: "user-is-admin",
            kind: ErrorKind::Missing,
        };
        try_from_authenticated_invalid_is_admin(
            "User-Session" => "authenticated",
            "User-ID" => "55",
            "User-Given-Name" => "John",
            "User-Family-Name" => "Doe",
            "User-Email" => "john.doe@gmail.com",
            "User-Role" => "organizer",
            "User-Is-Admin" => "0",
        ) => {
            header: "user-is-admin",
            kind: ErrorKind::Error(_),
        };
    }

    #[test]
    fn try_from_authenticated_given_name_accepts_utf8() {
        let headers = headers! {
            "User-Session" => "authenticated",
            "User-ID" => "55",
            "User-Given-Name" => "Jöhn",
            "User-Family-Name" => "Doe",
            "User-Email" => "john.doe@gmail.com",
            "User-Role" => "organizer",
            "User-Is-Admin" => "true",
        };

        let context = User::try_from(&headers).unwrap();
        let User::Authenticated(context) = context else {
            panic!("expected Context::Authenticated, got {:?}", context);
        };
        assert_eq!(context.given_name, "Jöhn");
    }

    #[test]
    fn try_from_authenticated_family_name_accepts_utf8() {
        let headers = headers! {
            "User-Session" => "authenticated",
            "User-ID" => "55",
            "User-Given-Name" => "John",
            "User-Family-Name" => "Döe",
            "User-Email" => "john.doe@gmail.com",
            "User-Role" => "organizer",
            "User-Is-Admin" => "true",
        };

        let context = User::try_from(&headers).unwrap();
        let User::Authenticated(context) = context else {
            panic!("expected Context::Authenticated, got {:?}", context);
        };
        assert_eq!(context.family_name, "Döe");
    }

    #[test]
    fn try_from_authenticated_email_accepts_utf8() {
        let headers = headers! {
            "User-Session" => "authenticated",
            "User-ID" => "55",
            "User-Given-Name" => "John",
            "User-Family-Name" => "Doe",
            "User-Email" => "jöhn.döe@gmail.cöm",
            "User-Role" => "organizer",
            "User-Is-Admin" => "true",
        };

        let context = User::try_from(&headers).unwrap();
        let User::Authenticated(context) = context else {
            panic!("expected Context::Authenticated, got {:?}", context);
        };
        assert_eq!(context.email, "jöhn.döe@gmail.cöm");
    }

    #[test]
    fn try_from_authenticated_role_is_optional() {
        let headers = headers! {
            "User-Session" => "authenticated",
            "User-ID" => "55",
            "User-Given-Name" => "John",
            "User-Family-Name" => "Doe",
            "User-Email" => "john.doe@gmail.cöm",
            "User-Is-Admin" => "true",
        };

        let context = User::try_from(&headers).unwrap();
        let User::Authenticated(context) = context else {
            panic!("expected Context::Authenticated, got {:?}", context);
        };
        assert_eq!(context.role, None);
    }

    #[test]
    fn into_headers_unauthenticated() {
        let headers = User::Unauthenticated.into_headers();
        assert_eq!(headers.get("user-session").unwrap(), "unauthenticated");
    }

    #[test]
    fn into_headers_oauth() {
        let headers = User::OAuth.into_headers();
        assert_eq!(headers.get("user-session").unwrap(), "oauth");
    }

    #[test]
    fn into_headers_registration_needed() {
        let context = User::RegistrationNeeded(UserRegistrationNeeded {
            provider: String::from("google"),
            id: String::from("01234567890"),
            email: String::from("hello@world.com"),
        });
        let headers = context.into_headers();

        assert_eq!(headers.get("user-session").unwrap(), "registration-needed");
        assert_eq!(headers.get("oauth-provider-slug").unwrap(), "google");
        assert_eq!(headers.get("oauth-user-id").unwrap(), "01234567890");
        assert_eq!(headers.get("oauth-user-email").unwrap(), "hello@world.com");
    }

    #[test]
    fn into_headers_authenticated() {
        let context = User::Authenticated(AuthenticatedUser {
            id: 79,
            given_name: String::from("John"),
            family_name: String::from("Doe"),
            email: String::from("john.doe@gmail.com"),
            role: Some(UserRole::Manager),
            is_admin: false,
        });
        let headers = context.into_headers();

        assert_eq!(headers.get("user-session").unwrap(), "authenticated");
        assert_eq!(headers.get("user-id").unwrap(), "79");
        assert_eq!(headers.get("user-given-name").unwrap(), "John");
        assert_eq!(headers.get("user-family-name").unwrap(), "Doe");
        assert_eq!(headers.get("user-email").unwrap(), "john.doe@gmail.com");
        assert_eq!(headers.get("user-is-admin").unwrap(), "false");
    }

    macro_rules! test_roundtrip {
        ( $(
            $name:ident ( $context:expr )
        );+ $(;)? ) => {
            $(
                #[test]
                fn $name() {
                    let context = $context;

                    let headers = context.clone().into_headers();
                    let roundtripped = User::try_from(&headers).unwrap();
                    assert_eq!(context, roundtripped);
                }
            )+
        };
    }

    test_roundtrip! {
        roundtrip_unauthenticated(User::Unauthenticated);
        roundtrip_oauth(User::OAuth);
        roundtrip_registration_needed(User::RegistrationNeeded(UserRegistrationNeeded {
            provider: String::from("google"),
            id: String::from("01234567890"),
            email: String::from("hellö@wörld.cöm"),
        }));
        roundtrip_authenticated(User::Authenticated(AuthenticatedUser {
            id: 79,
            given_name: String::from("Jöhn"),
            family_name: String::from("Döe"),
            email: String::from("jöhn.döe@gmail.cöm"),
            role: Some(UserRole::Participant),
            is_admin: false,
        }));
    }

    #[test]
    fn user_role_ordering() {
        assert!(UserRole::Director > UserRole::Manager);
        assert!(UserRole::Director > UserRole::Organizer);
        assert!(UserRole::Director > UserRole::Participant);
        assert!(UserRole::Manager > UserRole::Organizer);
        assert!(UserRole::Manager > UserRole::Participant);
        assert!(UserRole::Organizer > UserRole::Participant);
    }
}
