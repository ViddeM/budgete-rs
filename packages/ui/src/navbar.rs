use dioxus::prelude::*;

/// Top navigation bar. Expects a `Route` enum that implements `Routable`.
/// We keep this generic with `children` so the web crate can pass `Link` elements.
#[component]
pub fn Navbar(
    children: Element,
    dark_mode: bool,
    on_toggle: EventHandler<()>,
) -> Element {
    let mut nav_open = use_signal(|| false);
    let icon = if dark_mode { "☀" } else { "☾" };
    rsx! {
        nav {
            class: "app-nav",
            span { class: "nav-brand", "Budget" }
            // Nav links: flex row on desktop, collapsible drawer on mobile
            div {
                class: if nav_open() { "nav-links nav-links--open" } else { "nav-links" },
                {children}
            }
            // Right-side controls: always visible
            div {
                class: "nav-controls",
                button {
                    class: "nav-theme-btn",
                    onclick: move |_| on_toggle.call(()),
                    "{icon}"
                }
                // Hamburger: only visible on mobile via CSS
                button {
                    class: "nav-hamburger",
                    onclick: move |_| *nav_open.write() = !nav_open(),
                    if nav_open() { "✕" } else { "☰" }
                }
            }
        }
    }
}

/// A single nav link styled for the dark navbar.
#[component]
pub fn NavLink(children: Element) -> Element {
    rsx! {
        span { class: "nav-link", {children} }
    }
}
