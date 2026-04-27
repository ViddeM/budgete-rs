use axum::{
    extract::Query,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use uuid::Uuid;

use super::config::oauth_config;
use super::session::{
    clear_session_cookie, create_session, delete_session, session_token_from_headers,
    set_session_cookie, upsert_user,
};

// ---------------------------------------------------------------------------
// GET /auth/login  — start the OAuth flow
// ---------------------------------------------------------------------------

pub async fn login_handler() -> Response {
    let cfg = oauth_config();

    // Generate a random state token to prevent CSRF.
    let state = Uuid::new_v4().to_string();

    let auth_url = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}",
        cfg.auth_url,
        urlenccode(&cfg.client_id),
        urlenccode(&cfg.redirect_url),
        urlenccode(&cfg.scopes),
        urlenccode(&state),
    );

    // Store state in a short-lived cookie so we can verify it on callback.
    let state_cookie = format!(
        "oauth_state={state}; Path=/; HttpOnly; SameSite=Lax; Max-Age=600"
    );

    (
        StatusCode::FOUND,
        [
            (header::LOCATION, auth_url),
            (header::SET_COOKIE, state_cookie),
        ],
    )
        .into_response()
}

// ---------------------------------------------------------------------------
// GET /auth/callback  — OAuth provider redirects here
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct CallbackParams {
    code: String,
    state: String,
}

pub async fn callback_handler(
    Query(params): Query<CallbackParams>,
    headers: axum::http::HeaderMap,
) -> Response {
    // Verify state to prevent CSRF.
    let stored_state = headers
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|part| {
                let part = part.trim();
                let (k, v) = part.split_once('=')?;
                if k.trim() == "oauth_state" { Some(v.trim().to_string()) } else { None }
            })
        });

    if stored_state.as_deref() != Some(params.state.as_str()) {
        return (StatusCode::BAD_REQUEST, "Invalid OAuth state").into_response();
    }

    let cfg = oauth_config();

    // Exchange authorization code for an access token.
    tracing::info!("OAuth callback: exchanging code for token");
    let token_res = match exchange_code(&cfg, &params.code).await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("OAuth token exchange failed: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Token exchange failed").into_response();
        }
    };

    // Fetch the user's identity from the userinfo endpoint.
    tracing::info!("OAuth callback: fetching userinfo");
    let userinfo = match fetch_userinfo(&cfg, &token_res.access_token).await {
        Ok(u) => u,
        Err(e) => {
            tracing::error!("OAuth userinfo fetch failed: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Userinfo fetch failed").into_response();
        }
    };

    // Upsert the user in the DB.
    tracing::info!("OAuth callback: upserting user sub={}", userinfo.sub);
    let user_id = match upsert_user(
        &cfg.provider_name,
        &userinfo.sub,
        userinfo.email.as_deref(),
        userinfo.name.as_deref(),
    )
    .await
    {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("Failed to upsert user: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    };

    // Create a session.
    tracing::info!("OAuth callback: creating session for user {user_id}");
    let token = match create_session(user_id, cfg.session_duration_hours).await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to create session: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Session creation failed").into_response();
        }
    };

    let max_age = cfg.session_duration_hours * 3600;
    let session_cookie = set_session_cookie(token, max_age);
    // Clear the oauth_state cookie.
    let clear_state = "oauth_state=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0".to_string();

    tracing::info!("OAuth callback: success — session {token} created, redirecting to /");

    // Build the response manually so we can set two Set-Cookie headers.
    axum::response::Response::builder()
        .status(StatusCode::FOUND)
        .header(header::LOCATION, "/")
        .header(header::SET_COOKIE, session_cookie)
        .header(header::SET_COOKIE, clear_state)
        .body(axum::body::Body::empty())
        .unwrap()
}

// ---------------------------------------------------------------------------
// GET /auth/logout
// ---------------------------------------------------------------------------

pub async fn logout_handler(headers: axum::http::HeaderMap) -> Response {
    if let Some(token) = session_token_from_headers(&headers) {
        let _ = delete_session(token).await;
    }
    axum::response::Response::builder()
        .status(StatusCode::FOUND)
        .header(header::LOCATION, "/login")
        .header(header::SET_COOKIE, clear_session_cookie())
        .body(axum::body::Body::empty())
        .unwrap()
}

// ---------------------------------------------------------------------------
// OAuth HTTP helpers
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct UserinfoResponse {
    /// Subject — the provider's stable user identifier.
    sub: String,
    email: Option<String>,
    name: Option<String>,
}

async fn exchange_code(
    cfg: &super::config::OAuthConfig,
    code: &str,
) -> Result<TokenResponse, reqwest::Error> {
    reqwest::Client::new()
        .post(&cfg.token_url)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", &cfg.redirect_url),
            ("client_id", &cfg.client_id),
            ("client_secret", &cfg.client_secret),
        ])
        .send()
        .await?
        .json::<TokenResponse>()
        .await
}

async fn fetch_userinfo(
    cfg: &super::config::OAuthConfig,
    access_token: &str,
) -> Result<UserinfoResponse, String> {
    let resp = reqwest::Client::new()
        .get(&cfg.userinfo_url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = resp.status();
    let body = resp.text().await.map_err(|e| e.to_string())?;

    if !status.is_success() {
        return Err(format!("userinfo endpoint returned {status}: {body}"));
    }

    serde_json::from_str::<UserinfoResponse>(&body).map_err(|e| {
        tracing::error!("Userinfo response body: {body}");
        format!("deserialize error: {e}")
    })
}

// ---------------------------------------------------------------------------
// Minimal URL percent-encoding for query string values
// ---------------------------------------------------------------------------

fn urlenccode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9'
            | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            b' ' => out.push('+'),
            _ => { let _ = std::fmt::Write::write_fmt(&mut out, format_args!("%{b:02X}")); }
        }
    }
    out
}
