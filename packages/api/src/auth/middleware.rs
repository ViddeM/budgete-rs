use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};

use super::session::session_token_from_headers;

/// Axum middleware that enforces authentication on every request except:
/// - `/api/*`    — server functions (auth via `current_household_id()`) + OAuth handlers
/// - `/login`    — the Dioxus login page
/// - `/assets/*` — static CSS / JS / images from the asset pipeline
/// - `/_dioxus/*` — Dioxus hot-reload and client bundle
/// - `/wasm/*`   — compiled Wasm + JS bundle served by dx
///
/// Authenticated users without a household are redirected to `/household/setup`.
/// When `LOCAL_MODE=true`, all requests pass through unconditionally.
pub async fn require_auth(request: Request, next: Next) -> Response {
    if crate::config::config().local_mode {
        return next.run(request).await;
    }

    let path = request.uri().path().to_owned();

    let is_public = path.starts_with("/api/")
        || path.starts_with("/assets/")
        || path.starts_with("/_dioxus/")
        || path.starts_with("/wasm/")
        || path == "/login";

    if is_public {
        return next.run(request).await;
    }

    // This path needs a session but not a household.
    let needs_only_auth = path == "/household/setup";

    let session_state = {
        let headers = request.headers();
        match session_token_from_headers(headers) {
            None => None,
            Some(token) => get_session_state(token).await.ok().flatten(),
        }
    };

    match session_state {
        None => redirect("/login"),
        Some((_user_id, None)) if !needs_only_auth => redirect("/household/setup"),
        Some(_) => next.run(request).await,
    }
}

/// Returns `Some((user_id, household_id))` for a valid, unexpired session token.
async fn get_session_state(
    token: uuid::Uuid,
) -> Result<Option<(uuid::Uuid, Option<uuid::Uuid>)>, sqlx::Error> {
    use crate::db::pool;
    let row: Option<(uuid::Uuid, Option<uuid::Uuid>)> = sqlx::query_as(
        "SELECT u.id, u.household_id \
         FROM sessions s \
         JOIN users u ON u.id = s.user_id \
         WHERE s.token = $1 AND s.expires_at > NOW()",
    )
    .bind(token)
    .fetch_optional(pool())
    .await?;
    Ok(row)
}

fn redirect(location: &'static str) -> Response {
    axum::response::Response::builder()
        .status(StatusCode::FOUND)
        .header(header::LOCATION, location)
        .body(axum::body::Body::empty())
        .unwrap()
}
