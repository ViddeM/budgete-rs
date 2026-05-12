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
const MAIN_CSS: Asset = asset!("/assets/main.css");

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

        use axum::{middleware, routing::get};
        use api::auth::{handlers, middleware::require_auth};

        // In local mode: ensure the built-in user row exists, skip OAuth routes.
        let router = if api::config::config().local_mode {
            api::auth::ensure_local_user()
                .await
                .expect("failed to ensure local user");

            dioxus::server::router(App)
        } else {
            dioxus::server::router(App)
                // OAuth flow — handled entirely by axum, outside Dioxus routing.
                .route("/api/auth/login",    get(handlers::login_handler))
                .route("/api/auth/callback", get(handlers::callback_handler))
                .route("/api/auth/logout",   get(handlers::logout_handler))
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
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
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
                style: "color: #94a3b8; text-decoration: none; font-size: 0.9rem;",
                "Dashboard"
            }
            Link {
                to: Route::Upload {},
                active_class: "nav-active",
                style: "color: #94a3b8; text-decoration: none; font-size: 0.9rem;",
                "Upload"
            }
            Link {
                to: Route::Classify {},
                active_class: "nav-active",
                style: "color: #94a3b8; text-decoration: none; font-size: 0.9rem;",
                "Classify"
            }
            Link {
                to: Route::Transactions {},
                active_class: "nav-active",
                style: "color: #94a3b8; text-decoration: none; font-size: 0.9rem;",
                "Transactions"
            }
            Link {
                to: Route::Projects {},
                active_class: "nav-active",
                style: "color: #94a3b8; text-decoration: none; font-size: 0.9rem;",
                "Projects"
            }
            Link {
                to: Route::Analytics {},
                active_class: "nav-active",
                style: "color: #94a3b8; text-decoration: none; font-size: 0.9rem;",
                "Analytics"
            }
            a {
                href: "/api/auth/logout",
                style: "color: #94a3b8; text-decoration: none; font-size: 0.9rem;",
                "Log out"
            }
        }
        Outlet::<Route> {}
    }
}
