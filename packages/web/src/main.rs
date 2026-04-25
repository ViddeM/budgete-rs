use dioxus::prelude::*;
use views::{Analytics, Classify, Dashboard, Upload};

mod views;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(AppLayout)]
        #[route("/")]
        Dashboard {},
        #[route("/upload")]
        Upload {},
        #[route("/classify")]
        Classify {},
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
        Ok(dioxus::server::router(App))
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
                to: Route::Analytics {},
                active_class: "nav-active",
                style: "color: #94a3b8; text-decoration: none; font-size: 0.9rem;",
                "Analytics"
            }
        }
        Outlet::<Route> {}
    }
}
