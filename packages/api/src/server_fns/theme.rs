use dioxus::prelude::*;

/// Returns `"dark"` or `"light"` based on the `theme` cookie sent with the
/// current server-function request.
///
/// Used by the `App` root component via `use_server_future` so that the
/// initial SSR render knows the user's preferred theme, avoiding a flash of
/// the wrong theme on first paint.
#[server]
pub async fn get_theme() -> Result<String, ServerFnError> {
    use dioxus::prelude::dioxus_fullstack::FullstackContext;

    let ctx = match FullstackContext::current() {
        Some(c) => c,
        None => return Ok("light".to_string()),
    };

    let headers = ctx.parts_mut().headers.clone();
    let cookie_str = match headers.get("cookie").and_then(|v| v.to_str().ok()) {
        Some(s) => s.to_string(),
        None => return Ok("light".to_string()),
    };

    for part in cookie_str.split(';') {
        let part = part.trim();
        if let Some((key, val)) = part.split_once('=') {
            if key.trim() == "theme" && val.trim() == "dark" {
                return Ok("dark".to_string());
            }
        }
    }

    Ok("light".to_string())
}
