//! `api` — server-side logic: database access, CSV parsing, and server functions.
//!
//! Only the `server_fns` and `models` modules are public; consumers (the `web`
//! and `ui` crates) should only import models and call the generated server-fn
//! stubs — never touch `db` or `csv` directly.

pub mod models;

#[cfg(feature = "server")]
pub mod db;

#[cfg(feature = "server")]
pub(crate) mod db_rows;

#[cfg(feature = "server")]
pub(crate) mod csv;

/// Auth: OAuth flow, session management, axum middleware.
#[cfg(feature = "server")]
pub mod auth;

/// Convenience re-export so web can call `api::current_user_id()` directly.
#[cfg(feature = "server")]
pub use auth::current_user_id;

pub mod server_fns;

// Re-export commonly used types and server function stubs at crate root.
pub use models::*;
pub use server_fns::analytics::*;
pub use server_fns::categories::*;
pub use server_fns::dashboard::*;
pub use server_fns::groups::*;
pub use server_fns::transactions::*;
