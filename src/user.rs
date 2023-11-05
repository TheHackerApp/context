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
    Authenticated(AuthenticatedContext)
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