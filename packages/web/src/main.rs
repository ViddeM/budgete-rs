use api::get_theme;
use dioxus::document::eval;
use dioxus::prelude::*;
use views::{Analytics, Classify, Dashboard, Login, Projects, Transactions, Upload};

mod views;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    // Login lives outside the app shell — no navbar, no layout.
    #[route("/login")]
    Login {},

    #[layout(AppLayout)]
        #[route("/")]
        Dashboard {},
        #[route("/upload")]
        Upload {},
        #[route("/classify")]
        Classify {},
        #[route("/transactions")]
        Transactions {},
        #[route("/projects")]
        Projects {},
        #[route("/analytics")]
        Analytics {},
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const FAVICON_SVG: Asset = asset!("/assets/favicon.svg");
const MAIN_CSS: Asset = asset!("/assets/main.css");

/// Inline script injected into <head> before any CSS renders.
/// Reads the `theme` cookie synchronously and sets `data-theme` on <html>,
/// preventing a flash of light mode before the Dioxus effect fires.
const THEME_INIT_SCRIPT: &str = r#"{const m=document.cookie.match(/(?:^|;)\s*theme=([^;]+)/);if(m&&m[1]==="dark")document.documentElement.setAttribute("data-theme","dark");}"#;

fn main() {
    // On the server, use dioxus::server::serve so that the DB pool is
    // initialised inside the dioxus-managed tokio runtime.  The closure is
    // re-called on every hot-patch, which is fine because init_pool() is
    // idempotent (OnceLock).
    #[cfg(feature = "server")]
    dioxus::server::serve(|| async {
        // Validate all environment variables first.  Panics immediately with a
        // clear message if any required variable is absent or a URL is invalid.
        api::config::load();

        api::db::init_pool()
            .await
            .expect("failed to initialise DB pool");

        use api::auth::{handlers, middleware::require_auth};
        use axum::{middleware, routing::get};

        // In local mode: ensure the built-in user row exists, skip OAuth routes.
        let router = if api::config::config().local_mode {
            api::auth::ensure_local_user()
                .await
                .expect("failed to ensure local user");

            dioxus::server::router(App)
                // Logout is still needed in local mode so the nav link works.
                // The handler safely no-ops when there is no session cookie.
                .route("/api/auth/logout", get(handlers::logout_handler))
        } else {
            dioxus::server::router(App)
                // OAuth flow — handled entirely by axum, outside Dioxus routing.
                .route("/api/auth/login", get(handlers::login_handler))
                .route("/api/auth/callback", get(handlers::callback_handler))
                .route("/api/auth/logout", get(handlers::logout_handler))
        };

        // Enforce authentication on every request (see middleware for
        // the list of public paths that are always allowed through, and for
        // the LOCAL_MODE bypass).
        let router = router.layer(middleware::from_fn(require_auth));

        Ok(router)
    });

    // On the client (wasm32) the server block above is compiled out.
    #[cfg(not(feature = "server"))]
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // Read initial theme from cookie on the server so SSR renders the correct
    // theme — the serialised value is sent to the client so the first
    // hydration render is also consistent (no flash).
    let theme_res = use_server_future(get_theme)?;
    let initial_dark = theme_res()
        .and_then(|r| r.ok())
        .map(|s| s == "dark")
        .unwrap_or(false);

    let dark_mode = use_signal(|| initial_dark);
    use_context_provider(|| dark_mode);

    // Sync data-theme on <html> and write the cookie whenever the signal changes.
    // This only runs in the browser (use_effect is client-only).
    use_effect(move || {
        let theme = if dark_mode() { "dark" } else { "light" };
        eval(&format!(
            "document.documentElement.setAttribute('data-theme','{theme}');\
             document.cookie='theme={theme};Path=/;Max-Age=31536000;SameSite=Lax';"
        ));
    });

    rsx! {
        document::Title { "Budgets" }
        document::Meta { name: "viewport", content: "width=device-width, initial-scale=1" }
        document::Link { rel: "icon", r#type: "image/svg+xml", href: FAVICON_SVG }
        document::Link { rel: "icon", r#type: "image/x-icon", sizes: "48x48", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Script { {THEME_INIT_SCRIPT} }
        Router::<Route> {}
    }
}

#[component]
fn AppLayout() -> Element {
    let mut dark_mode = use_context::<Signal<bool>>();

    rsx! {
        ui::Navbar {
            dark_mode: dark_mode(),
            on_toggle: move |_| *dark_mode.write() = !dark_mode(),
            Link {
                to: Route::Dashboard {},
                active_class: "nav-active",
                class: "nav-link",
                "Dashboard"
            }
            Link {
                to: Route::Upload {},
                active_class: "nav-active",
                class: "nav-link",
                "Upload"
            }
            Link {
                to: Route::Classify {},
                active_class: "nav-active",
                class: "nav-link",
                "Classify"
            }
            Link {
                to: Route::Transactions {},
                active_class: "nav-active",
                class: "nav-link",
                "Transactions"
            }
            Link {
                to: Route::Projects {},
                active_class: "nav-active",
                class: "nav-link",
                "Projects"
            }
            Link {
                to: Route::Analytics {},
                active_class: "nav-active",
                class: "nav-link",
                "Analytics"
            }
            a {
                href: "/api/auth/logout",
                class: "nav-link",
                "Log out"
            }
        }
        Outlet::<Route> {}
    }
}
