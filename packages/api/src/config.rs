//! Application configuration — all environment variables are read, validated,
//! and stored here exactly once at server startup.
//!
//! Call [`load()`] as the very first thing in `main` (before touching the DB
//! or any auth code).  If any required variable is absent or a URL is
//! syntactically invalid the process panics immediately with a clear message
//! that names the offending variable.
//!
//! After [`load()`] returns, every other module can call [`config()`] to
//! obtain a `&'static AppConfig` at zero cost.

use std::sync::OnceLock;

static APP_CONFIG: OnceLock<AppConfig> = OnceLock::new();

// ---------------------------------------------------------------------------
// Public structs
// ---------------------------------------------------------------------------

/// Fully-validated application configuration loaded from the environment.
pub struct AppConfig {
    /// PostgreSQL connection URL (`DATABASE_URL`).
    pub database_url: String,
    /// When `true`, authentication is completely disabled.  All requests run
    /// as the built-in local user and the OAuth env vars are not required.
    /// Set `LOCAL_MODE=true` for single-user local deployments.
    pub local_mode: bool,
    /// OAuth 2.0 provider settings.  `None` when `local_mode` is `true`.
    pub oauth: Option<OAuthConfig>,
}

/// OAuth 2.0 provider configuration.
///
/// # Required environment variables (when `LOCAL_MODE` is not `true`)
///
/// | Variable               | Description                                              |
/// |------------------------|----------------------------------------------------------|
/// | `OAUTH_CLIENT_ID`      | Application client ID from your OAuth provider           |
/// | `OAUTH_CLIENT_SECRET`  | Application client secret                                |
/// | `OAUTH_AUTH_URL`       | Provider's authorization endpoint URL                    |
/// | `OAUTH_TOKEN_URL`      | Provider's token endpoint URL                            |
/// | `OAUTH_USERINFO_URL`   | Provider's userinfo endpoint URL                         |
/// | `OAUTH_REDIRECT_URL`   | Your app's callback URL (must be registered with the provider) |
///
/// # Optional environment variables
///
/// | Variable                 | Default                  |
/// |--------------------------|--------------------------|
/// | `OAUTH_SCOPES`           | `"openid email profile"` |
/// | `OAUTH_PROVIDER_NAME`    | `"oauth"`                |
/// | `SESSION_DURATION_HOURS` | `"720"` (30 days)        |
pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    /// Provider's authorization endpoint — the browser is redirected here.
    pub auth_url: String,
    /// Provider's token endpoint — server POSTs here to exchange the code.
    pub token_url: String,
    /// Provider's userinfo endpoint — server GETs here with the access token.
    pub userinfo_url: String,
    /// Callback URL registered with the provider.
    pub redirect_url: String,
    /// Space-separated OAuth scopes.
    pub scopes: String,
    /// Arbitrary label stored in the `users.provider` column (e.g. `"google"`).
    pub provider_name: String,
    /// How many hours a session token remains valid.
    pub session_duration_hours: i64,
}

// ---------------------------------------------------------------------------
// Loading & validation
// ---------------------------------------------------------------------------

impl AppConfig {
    fn from_env() -> Self {
        // Load .env if present; ignore errors (file may not exist in production).
        dotenvy::dotenv().ok();

        fn require(key: &str) -> String {
            std::env::var(key)
                .unwrap_or_else(|_| panic!("Missing required environment variable: {key}"))
        }

        fn optional(key: &str, default: &str) -> String {
            std::env::var(key).unwrap_or_else(|_| default.to_string())
        }

        /// Read a required env var and validate that it is a well-formed URL.
        fn require_url(key: &str) -> String {
            let raw = require(key);
            url::Url::parse(&raw).unwrap_or_else(|e| {
                panic!("Environment variable {key}={raw:?} is not a valid URL: {e}")
            });
            raw
        }

        let local_mode = optional("LOCAL_MODE", "false")
            .parse::<bool>()
            .expect("LOCAL_MODE must be 'true' or 'false'");

        let oauth = if local_mode {
            tracing::info!("LOCAL_MODE=true — authentication disabled, running as local user");
            None
        } else {
            Some(OAuthConfig {
                client_id: require("OAUTH_CLIENT_ID"),
                client_secret: require("OAUTH_CLIENT_SECRET"),
                auth_url: require_url("OAUTH_AUTH_URL"),
                token_url: require_url("OAUTH_TOKEN_URL"),
                userinfo_url: require_url("OAUTH_USERINFO_URL"),
                redirect_url: require_url("OAUTH_REDIRECT_URL"),
                scopes: optional("OAUTH_SCOPES", "openid email profile"),
                provider_name: optional("OAUTH_PROVIDER_NAME", "oauth"),
                session_duration_hours: optional("SESSION_DURATION_HOURS", "720")
                    .parse()
                    .expect("SESSION_DURATION_HOURS must be a valid integer"),
            })
        };

        AppConfig {
            database_url: require_url("DATABASE_URL"),
            local_mode,
            oauth,
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Validate and load all configuration from environment variables.
///
/// Panics immediately with a descriptive message if:
/// - any required variable is absent, or
/// - any URL-typed variable does not parse as a valid URL.
///
/// Safe to call multiple times — only the first call has any effect.
pub fn load() {
    APP_CONFIG.get_or_init(AppConfig::from_env);
}

/// Return a reference to the validated application configuration.
///
/// Panics if [`load()`] has not been called first.
#[inline]
pub fn config() -> &'static AppConfig {
    APP_CONFIG
        .get()
        .expect("app config not loaded — call config::load() at server startup")
}
