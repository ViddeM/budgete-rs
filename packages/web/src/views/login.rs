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
            style: "display:flex; align-items:center; justify-content:center; min-height:100vh; background:#0f172a;",
            div {
                style: "text-align:center; padding:2rem;",
                h1 {
                    style: "color:#f1f5f9; font-size:2rem; font-weight:700; margin-bottom:0.5rem; letter-spacing:-0.02em;",
                    "Budgete"
                }

                if is_local {
                    p {
                        style: "color:#94a3b8; margin-bottom:2.5rem; font-size:1rem;",
                        "Local mode is active — no sign-in required."
                    }
                    a {
                        href: "/",
                        style: "display:inline-block; background:#6366f1; color:#fff; padding:0.75rem 2.5rem; border-radius:0.5rem; text-decoration:none; font-weight:600; font-size:1rem;",
                        "Open the app"
                    }
                } else {
                    p {
                        style: "color:#94a3b8; margin-bottom:2.5rem; font-size:1rem;",
                        "Track your spending — sign in to continue."
                    }
                    a {
                        href: "/api/auth/login",
                        style: "display:inline-block; background:#6366f1; color:#fff; padding:0.75rem 2.5rem; border-radius:0.5rem; text-decoration:none; font-weight:600; font-size:1rem; transition:background 0.15s;",
                        "Sign in"
                    }
                }
            }
        }
    }
}
