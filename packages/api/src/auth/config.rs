/// OAuth 2.0 provider configuration.
///
/// Fill in the values below (or set the corresponding environment variables)
/// for your chosen OAuth provider.  The application reads environment variables
/// at runtime; the constants in `defaults` are used as fallback documentation.
///
/// # Required environment variables
///
/// | Variable                  | Description                                         |
/// |---------------------------|-----------------------------------------------------|
/// | `OAUTH_CLIENT_ID`         | Application client ID from your OAuth provider      |
/// | `OAUTH_CLIENT_SECRET`     | Application client secret                           |
/// | `OAUTH_AUTH_URL`          | Provider's authorization endpoint URL               |
/// | `OAUTH_TOKEN_URL`         | Provider's token endpoint URL                       |
/// | `OAUTH_USERINFO_URL`      | Provider's userinfo endpoint URL                    |
 /// | `OAUTH_REDIRECT_URL`      | Your app's callback URL (must be registered with provider) |
/// | `OAUTH_SCOPES`            | Space-separated list of scopes (default: `openid email profile`) |
/// | `OAUTH_PROVIDER_NAME`     | Arbitrary label stored with the user (e.g. `google`) |
/// | `SESSION_DURATION_HOURS`  | How many hours a session stays valid (default: `720`) |
///
/// # Example .env entries
///
/// ```env
/// OAUTH_CLIENT_ID=your-client-id
/// OAUTH_CLIENT_SECRET=your-client-secret
/// OAUTH_AUTH_URL=https://accounts.google.com/o/oauth2/v2/auth
/// OAUTH_TOKEN_URL=https://oauth2.googleapis.com/token
/// OAUTH_USERINFO_URL=https://openidconnect.googleapis.com/v1/userinfo
 /// OAUTH_REDIRECT_URL=http://localhost:8080/auth/callback
/// OAUTH_SCOPES=openid email profile
/// OAUTH_PROVIDER_NAME=google
/// ```

pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    /// Provider's authorization endpoint — browser is redirected here to log in.
    pub auth_url: String,
    /// Provider's token endpoint — server POSTs here to exchange the code.
    pub token_url: String,
    /// Provider's userinfo endpoint — server GETs here with the access token.
    pub userinfo_url: String,
    /// The URL that the provider should redirect the browser to after login.
    /// Must be registered in your provider's application settings.
    pub redirect_url: String,
    /// Space-separated OAuth scopes requested from the provider.
    pub scopes: String,
    /// Arbitrary label stored in the `users.provider` column (e.g. `"google"`).
    pub provider_name: String,
    /// How many hours a session token remains valid.
    pub session_duration_hours: i64,
}

/// Load OAuth configuration from environment variables.
/// Panics at startup if required variables are missing.
pub fn oauth_config() -> OAuthConfig {
    fn require(key: &str) -> String {
        std::env::var(key).unwrap_or_else(|_| panic!("Missing required env var: {key}"))
    }
    fn optional(key: &str, default: &str) -> String {
        std::env::var(key).unwrap_or_else(|_| default.to_string())
    }

    OAuthConfig {
        client_id:              require("OAUTH_CLIENT_ID"),
        client_secret:          require("OAUTH_CLIENT_SECRET"),
        auth_url:               require("OAUTH_AUTH_URL"),
        token_url:              require("OAUTH_TOKEN_URL"),
        userinfo_url:           require("OAUTH_USERINFO_URL"),
        redirect_url:           require("OAUTH_REDIRECT_URL"),
        scopes:                 optional("OAUTH_SCOPES", "openid email profile"),
        provider_name:          optional("OAUTH_PROVIDER_NAME", "oauth"),
        session_duration_hours: optional("SESSION_DURATION_HOURS", "720")
            .parse()
            .expect("SESSION_DURATION_HOURS must be an integer"),
    }
}
