use api::get_is_local_mode;
use dioxus::prelude::*;

/// Full-page login screen shown to unauthenticated visitors.
///
/// When `LOCAL_MODE=true` the server never redirects here, but if someone
/// navigates manually we show a "local mode active" banner instead of the
/// OAuth button.
#[component]
pub fn Login() -> Element {
    let local_mode = use_server_future(get_is_local_mode)?;

    let is_local = matches!(local_mode(), Some(Ok(true)));

    rsx! {
        div {
            class: "login-page",
            div {
                class: "login-box",
                h1 { class: "login-title", "BudgeteRS" }

                if is_local {
                    p { class: "login-sub", "Local mode is active — no sign-in required." }
                    a {
                        href: "/",
                        class: "login-btn",
                        "Open the app"
                    }
                } else {
                    p { class: "login-sub", "Track your spending — sign in to continue." }
                    a {
                        href: "/api/auth/login",
                        class: "login-btn",
                        "Sign in"
                    }
                }
            }
        }
    }
}
