use axum::http::HeaderMap;
use dioxus::prelude::ServerFnError;
use uuid::Uuid;

use crate::db::pool;

// ---------------------------------------------------------------------------
// Cookie helpers
// ---------------------------------------------------------------------------

const SESSION_COOKIE: &str = "session";

/// Parse the session token UUID from the `Cookie` request header.
pub(crate) fn session_token_from_headers(headers: &HeaderMap) -> Option<Uuid> {
    let cookie_header = headers.get("cookie")?.to_str().ok()?;
    cookie_header
        .split(';')
        .find_map(|part| {
            let part = part.trim();
            let (key, val) = part.split_once('=')?;
            if key.trim() == SESSION_COOKIE {
                Uuid::parse_str(val.trim()).ok()
            } else {
                None
            }
        })
}

/// Build a `Set-Cookie` header value that sets the session cookie.
pub(crate) fn set_session_cookie(token: Uuid, max_age_secs: i64) -> String {
    format!(
        "{SESSION_COOKIE}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age_secs}"
    )
}

/// Build a `Set-Cookie` header value that clears the session cookie.
pub(crate) fn clear_session_cookie() -> String {
    format!("{SESSION_COOKIE}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0")
}

// ---------------------------------------------------------------------------
// Database operations
// ---------------------------------------------------------------------------

/// Find or create a user from OAuth provider data.  Returns the user's UUID.
pub(crate) async fn upsert_user(
    provider: &str,
    provider_id: &str,
    email: Option<&str>,
    name: Option<&str>,
) -> Result<Uuid, sqlx::Error> {
    let db = pool();
    let id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO users (provider, provider_id, email, name)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (provider, provider_id)
        DO UPDATE SET
            email = COALESCE(EXCLUDED.email, users.email),
            name  = COALESCE(EXCLUDED.name,  users.name)
        RETURNING id
        "#,
    )
    .bind(provider)
    .bind(provider_id)
    .bind(email)
    .bind(name)
    .fetch_one(db)
    .await?;
    Ok(id)
}

/// Create a new session for `user_id` with the configured lifetime.
/// Returns the session token UUID to be stored in the client's cookie.
pub(crate) async fn create_session(
    user_id: Uuid,
    duration_hours: i64,
) -> Result<Uuid, sqlx::Error> {
    let db = pool();
    let token: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO sessions (user_id, expires_at)
        VALUES ($1, NOW() + ($2 * INTERVAL '1 hour'))
        RETURNING token
        "#,
    )
    .bind(user_id)
    .bind(duration_hours)
    .fetch_one(db)
    .await?;
    Ok(token)
}

/// Look up `user_id` for a session token.  Returns `None` if the token is
/// unknown or has expired.
pub(crate) async fn get_user_id_from_token(
    token: Uuid,
) -> Result<Option<Uuid>, sqlx::Error> {
    let db = pool();
    let user_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT user_id FROM sessions WHERE token = $1 AND expires_at > NOW()",
    )
    .bind(token)
    .fetch_optional(db)
    .await?;
    Ok(user_id)
}

/// Delete a session (logout).
pub(crate) async fn delete_session(token: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM sessions WHERE token = $1")
        .bind(token)
        .execute(pool())
        .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Server-function helper
// ---------------------------------------------------------------------------

/// Extract the authenticated user's UUID from the current server-function
/// request.  Returns a `ServerFnError` if the request has no valid session,
/// which propagates to the client as an error response.
pub async fn current_user_id() -> Result<Uuid, ServerFnError> {
    use dioxus::prelude::dioxus_fullstack::FullstackContext;

    let ctx = FullstackContext::current()
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let headers = ctx.parts_mut().headers.clone();
    let token = session_token_from_headers(&headers)
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    get_user_id_from_token(token)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("Session expired or invalid"))
}
