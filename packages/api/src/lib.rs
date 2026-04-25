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

pub mod server_fns;

// Re-export commonly used server function types at crate root for convenience.
pub use models::*;
pub use server_fns::categories::*;
pub use server_fns::dashboard::*;
pub use server_fns::groups::*;
pub use server_fns::transactions::*;
