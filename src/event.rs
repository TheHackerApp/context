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
