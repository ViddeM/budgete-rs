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
        api::db::init_pool()
            .await
            .expect("failed to initialise DB pool");

        use axum::{middleware, routing::get};
        use api::auth::{handlers, middleware::require_auth};

        let router = dioxus::server::router(App)
            // OAuth flow — handled entirely by axum, outside Dioxus routing.
            .route("/api/auth/login",    get(handlers::login_handler))
            .route("/api/auth/callback", get(handlers::callback_handler))
            .route("/api/auth/logout",   get(handlers::logout_handler))
            // Enforce authentication on every request (see middleware for
            // the list of public paths that are always allowed through).
            .layer(middleware::from_fn(require_auth));

        Ok(router)
    });

    // On the client (wasm32) the server block above is compiled out.
    #[cfg(not(feature = "server"))]
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        Router::<Route> {}
    }
}

#[component]
fn AppLayout() -> Element {
    rsx! {
        ui::Navbar {
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
                style: "color: #94a3b8; text-decoration: none; font-size: 0.9rem; margin-left: auto;",
                "Log out"
            }
        }
        Outlet::<Route> {}
    }
}
