use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::config::config;

/// Returns `true` when the server was started with `LOCAL_MODE=true`.
///
/// Used by the login page to show an appropriate message instead of the OAuth
/// sign-in button.
#[server]
pub async fn get_is_local_mode() -> Result<bool, ServerFnError> {
    Ok(config().local_mode)
}
