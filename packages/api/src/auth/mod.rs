pub mod config;
pub mod handlers;
pub mod middleware;
pub mod session;

/// Convenience re-export for server functions.
pub use session::current_user_id;

/// Convenience re-export for server startup: ensures the local-mode user row
/// exists in the DB when `LOCAL_MODE=true`.
pub use session::ensure_local_user;
