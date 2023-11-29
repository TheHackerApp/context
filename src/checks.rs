use crate::{
    scope::{self, EventContext},
    user::{self, AuthenticatedContext},
};
use async_graphql::{Context, Error, ErrorExtensions, Result};

/// Create an [`async_graphql::Guard`] out of a check function
pub fn guard<F, R>(check: F) -> impl Fn(&Context<'_>) -> Result<()> + Send + Sync + 'static
where
    F: Fn(&Context<'_>) -> Result<R> + Send + Sync + 'static,
{
    move |ctx| check(ctx).map(|_| ())
}

/// An error raised when the user has invalid permissions
#[derive(Debug)]
pub struct Forbidden;

impl From<Forbidden> for Error {
    fn from(_: Forbidden) -> Self {
        Error::new("forbidden").extend_with(|_, extensions| extensions.set("code", "FORBIDDEN"))
    }
}

/// Check if the requester is authenticated
pub fn is_authenticated<'c>(ctx: &'c Context) -> Result<&'c AuthenticatedContext> {
    let user = ctx.data_unchecked::<user::Context>();

    match user {
        user::Context::Authenticated(context) => Ok(context),
        _ => Err(Forbidden.into()),
    }
}

/// Check if the request was scoped to an user
pub fn is_user(ctx: &Context<'_>) -> Result<()> {
    let scope = ctx.data_unchecked::<scope::Context>();

    match scope {
        scope::Context::User => Ok(()),
        _ => Err(Forbidden.into()),
    }
}

/// Check if the request was scoped to an event
pub fn is_event<'c>(ctx: &Context<'c>) -> Result<&'c EventContext> {
    let scope = ctx.data_unchecked::<scope::Context>();

    match scope {
        scope::Context::Event(context) => Ok(context),
        _ => Err(Forbidden.into()),
    }
}

/// Check if the requester is an administrator
pub fn is_admin(ctx: &Context<'_>) -> Result<()> {
    let user = is_authenticated(ctx)?;

    if user.is_admin {
        Ok(())
    } else {
        Err(Forbidden.into())
    }
}

/// Ensures only admins can access a resource
pub fn admin_only(ctx: &Context<'_>) -> Result<()> {
    is_admin(ctx)?;

    let scope = ctx.data_unchecked::<scope::Context>();
    match scope {
        scope::Context::Admin => Ok(()),
        _ => Err(Forbidden.into()),
    }
}
