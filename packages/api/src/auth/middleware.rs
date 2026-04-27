use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};

use super::session::{get_user_id_from_token, session_token_from_headers};

/// Axum middleware that enforces authentication on every request except:
/// - `/api/*`    — server functions (auth via `current_user_id()`) + OAuth handlers
/// - `/login`    — the Dioxus login page
/// - `/assets/*` — static CSS / JS / images from the asset pipeline
/// - `/_dioxus/*` — Dioxus hot-reload and client bundle
/// - `/wasm/*`   — compiled Wasm + JS bundle served by dx
///
/// Unauthenticated requests to protected routes receive a `302 → /login`.
pub async fn require_auth(request: Request, next: Next) -> Response {
    let path = request.uri().path().to_owned();

    let is_public = path.starts_with("/api/")
        || path.starts_with("/assets/")
        || path.starts_with("/_dioxus/")
        || path.starts_with("/wasm/")
        || path == "/login";

    if is_public {
        return next.run(request).await;
    }

    let authenticated = {
        let headers = request.headers();
        match session_token_from_headers(headers) {
            None => {
                tracing::debug!("require_auth: no session cookie for {path}");
                false
            }
            Some(token) => match get_user_id_from_token(token).await {
                Ok(Some(_)) => true,
                Ok(None) => {
                    tracing::warn!("require_auth: session {token} not found or expired");
                    false
                }
                Err(e) => {
                    tracing::error!("require_auth: DB error checking session: {e}");
                    false
                }
            },
        }
    };

    if authenticated {
        next.run(request).await
    } else {
        axum::response::Response::builder()
            .status(StatusCode::FOUND)
            .header(header::LOCATION, "/login")
            .body(axum::body::Body::empty())
            .unwrap()
    }
}
