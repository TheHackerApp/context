use headers::{Error, Header, HeaderName, HeaderValue};
use std::iter;

static EVENT_SLUG: HeaderName = HeaderName::from_static("event-slug");
static EVENT_ORGANIZATION_ID: HeaderName = HeaderName::from_static("event-organization-id");

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

        impl ::std::convert::AsRef<$shared> for $target {
            fn as_ref(&self) -> &$shared {
                &self.0
            }
        }

        impl ::std::borrow::Borrow<$shared> for $target {
            fn borrow(&self) -> &$shared {
                &self.0
            }
        }

        impl ::std::ops::Deref for $target {
            type Target = $shared;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    };
}

/// `Event-Slug` header containing the event's slug
#[derive(Debug)]
pub struct EventSlug(String);

expose_inner!(EventSlug(shared: str, owned: String));

impl Header for EventSlug {
    fn name() -> &'static HeaderName {
        &EVENT_SLUG
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

/// `Event-Organization-ID` header containing the ID of the organization that runs the event
#[derive(Debug)]
pub struct EventOrganizationId(i32);

expose_inner!(EventOrganizationId(i32));

impl Header for EventOrganizationId {
    fn name() -> &'static HeaderName {
        &EVENT_ORGANIZATION_ID
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
