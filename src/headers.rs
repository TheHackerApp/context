use headers::{Error, Header, HeaderName, HeaderValue};
use std::{borrow::Borrow, iter, ops::Deref};

static EVENT_DOMAIN: HeaderName = HeaderName::from_static("event-domain");
static EVENT_SLUG: HeaderName = HeaderName::from_static("event-slug");
static EVENT_ORGANIZATION_ID: HeaderName = HeaderName::from_static("event-organization-id");
static USER_SESSION: HeaderName = HeaderName::from_static("user-session");
static OAUTH_PROVIDER_SLUG: HeaderName = HeaderName::from_static("oauth-provider-slug");
static OAUTH_USER_ID: HeaderName = HeaderName::from_static("oauth-user-id");
static OAUTH_USER_EMAIL: HeaderName = HeaderName::from_static("oauth-user-email");
static REQUEST_SCOPE: HeaderName = HeaderName::from_static("request-scope");
static USER_ID: HeaderName = HeaderName::from_static("user-id");
static USER_GIVEN_NAME: HeaderName = HeaderName::from_static("user-given-name");
static USER_FAMILY_NAME: HeaderName = HeaderName::from_static("user-family-name");
static USER_EMAIL: HeaderName = HeaderName::from_static("user-email");
static USER_IS_ADMIN: HeaderName = HeaderName::from_static("user-is-admin");

macro_rules! expose_inner {
    ( $target:ident ( $as:ty ) ) => {
        expose_inner!($target ( shared: $as, owned: $as ));
    };
    ( $target:ident ( shared: $shared:ty, owned: $owned:ty ) ) => {
        impl $target {
            /// Unwrap the header value
            pub fn into_inner(self) -> $owned {
                self.0
            }
        }

        impl AsRef<$shared> for $target {
            fn as_ref(&self) -> &$shared {
                &self.0
            }
        }

        impl Borrow<$shared> for $target {
            fn borrow(&self) -> &$shared {
                &self.0
            }
        }

        impl Deref for $target {
            type Target = $shared;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    };
}

macro_rules! text_header {
    (
        $( #[$attr:meta] )*
        ascii $name:ident, $header_name:ident
    ) => {
        text_header!(@internal
            $( #[$attr] )*
            $name
        );

        impl Header for $name {
            fn name() -> &'static HeaderName {
                &$header_name
            }

            fn decode<'i, I>(values: &mut I) -> Result<Self, Error>
            where
                I: Iterator<Item = &'i HeaderValue>,
            {
                let value = values.next().ok_or_else(Error::invalid)?;
                let decoded = value.to_str().map_err(|_| Error::invalid())?;

                Ok(Self(decoded.to_owned()))
            }

            fn encode<E>(&self, values: &mut E)
            where
                E: Extend<HeaderValue>,
            {
                let value = HeaderValue::try_from(&self.0).expect("must be valid ascii");
                values.extend(iter::once(value))
            }
        }
    };
    (
        $( #[$attr:meta] )*
        utf8 $name:ident, $header_name:ident
    ) => {
        text_header!(@internal
            $( #[$attr] )*
            $name
        );

        impl Header for $name {
            fn name() -> &'static HeaderName {
                &$header_name
            }

            fn decode<'i, I>(values: &mut I) -> Result<Self, Error>
            where
                I: Iterator<Item = &'i HeaderValue>,
            {
                let value = values.next().ok_or_else(Error::invalid)?.as_bytes();
                let decoded = String::from_utf8(value.to_vec()).map_err(|_| Error::invalid())?;

                Ok(Self(decoded))
            }

            fn encode<E>(&self, values: &mut E)
            where
                E: Extend<HeaderValue>,
            {
                let value = HeaderValue::from_bytes(&self.0.as_bytes()).expect("must be valid bytes");
                values.extend(iter::once(value))
            }
        }
    };
    (@internal
        $( #[$attr:meta] )*
        $name:ident
    ) => {
        $( #[$attr] )*
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct $name(String);

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        expose_inner!($name(shared: str, owned: String));
    };
}

macro_rules! int_header {
    (
        $( #[$attr:meta] )*
        $name:ident, $header_name:ident
    ) => {
        $( #[$attr] )*
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub struct $name(i32);

        expose_inner!($name(i32));

        impl From<i32> for $name {
            fn from(value: i32) -> Self {
                Self(value)
            }
        }

        impl Header for $name {
            fn name() -> &'static HeaderName {
                &$header_name
            }

            fn decode<'i, I>(values: &mut I) -> Result<Self, Error>
            where
                I: Iterator<Item = &'i HeaderValue>,
            {
                let value = values.next().ok_or_else(Error::invalid)?;
                let decoded = value
                    .to_str()
                    .map_err(|_| Error::invalid())?
                    .parse()
                    .map_err(|_| Error::invalid())?;

                Ok(Self(decoded))
            }

            fn encode<E>(&self, values: &mut E)
            where
                E: Extend<HeaderValue>,
            {
                let value = HeaderValue::from(self.0);
                values.extend(iter::once(value))
            }
        }
    };
}

text_header! {
    /// `Event-Domain` header containing a domain where the event can be found
    ascii EventDomain, EVENT_DOMAIN
}

text_header! {
    /// `Event-Slug` header containing the event's slug
    ascii EventSlug, EVENT_SLUG
}

int_header! {
    /// `Event-Organization-ID` header containing the ID of the organization that runs the event
    EventOrganizationId, EVENT_ORGANIZATION_ID
}

/// `User-Session` header containing the user's authentication status
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserSession {
    /// The user is unauthenticated
    Unauthenticated,
    /// The user is logging in
    OAuth,
    /// The user needs to complete registration
    RegistrationNeeded,
    /// The user is fully authenticated
    Authenticated,
}

impl Header for UserSession {
    fn name() -> &'static HeaderName {
        &USER_SESSION
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        let value = values.next().ok_or_else(Error::invalid)?;

        match value.as_bytes() {
            b"unauthenticated" => Ok(Self::Unauthenticated),
            b"oauth" => Ok(Self::OAuth),
            b"registration-needed" => Ok(Self::RegistrationNeeded),
            b"authenticated" => Ok(Self::Authenticated),
            _ => Err(Error::invalid()),
        }
    }

    fn encode<E>(&self, values: &mut E)
    where
        E: Extend<HeaderValue>,
    {
        let value = HeaderValue::from_static(match self {
            Self::Unauthenticated => "unauthenticated",
            Self::OAuth => "oauth",
            Self::RegistrationNeeded => "registration-needed",
            Self::Authenticated => "authenticated",
        });

        values.extend(iter::once(value))
    }
}

text_header! {
    /// `OAuth-Provider-Slug` header containing the slug of the provider the user used to
    /// authenticate with
    ascii OAuthProviderSlug, OAUTH_PROVIDER_SLUG
}

text_header! {
    /// `OAuth-User-ID` header containing the user's ID according to the provider
    ascii OAuthUserId, OAUTH_USER_ID
}

text_header! {
    /// `OAuth-User-Email` header containing the user's email according to the OAuth provider
    utf8 OAuthUserEmail, OAUTH_USER_EMAIL
}

/// `Request-Scope` header containing the desired scope for the request
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequestScope {
    /// A request with no restrictions on data access
    Admin,
    /// A request restricted to the current user
    ///
    /// This includes actions like authenticating and selecting an event
    User,
    /// A request restricted to the current event
    ///
    /// This includes actions like managing an event or submitting an application
    Event,
}

impl Header for RequestScope {
    fn name() -> &'static HeaderName {
        &REQUEST_SCOPE
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, Error>
    where
        Self: Sized,
        I: Iterator<Item = &'i HeaderValue>,
    {
        let value = values.next().ok_or_else(Error::invalid)?;

        match value.as_bytes() {
            b"admin" => Ok(Self::Admin),
            b"user" => Ok(Self::User),
            b"event" => Ok(Self::Event),
            _ => Err(Error::invalid()),
        }
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        let value = HeaderValue::from_static(match self {
            Self::Admin => "admin",
            Self::User => "user",
            Self::Event => "event",
        });

        values.extend(iter::once(value))
    }
}

text_header! {
    /// `User-Given-Name` header containing the user's given/first name
    utf8 UserGivenName, USER_GIVEN_NAME
}

text_header! {
    /// `User-Family-Name` header containing the user's family/last name
    utf8 UserFamilyName, USER_FAMILY_NAME
}

text_header! {
    /// `User-Email` header containing the user's email
    utf8 UserEmail, USER_EMAIL
}

int_header! {
    /// `User-ID` header containing the user's ID
    UserId, USER_ID
}

/// `User-Is-Admin` header containing whether the user is an admin
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UserIsAdmin(bool);

impl From<bool> for UserIsAdmin {
    fn from(value: bool) -> Self {
        Self(value)
    }
}

expose_inner!(UserIsAdmin(bool));

impl Header for UserIsAdmin {
    fn name() -> &'static HeaderName {
        &USER_IS_ADMIN
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        let value = values.next().ok_or_else(Error::invalid)?;

        match value.as_bytes() {
            b"true" => Ok(Self(true)),
            b"false" => Ok(Self(false)),
            _ => Err(Error::invalid()),
        }
    }

    fn encode<E>(&self, values: &mut E)
    where
        E: Extend<HeaderValue>,
    {
        let value = HeaderValue::from_static(if self.0 { "true" } else { "false" });
        values.extend(iter::once(value))
    }
}
