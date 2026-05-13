use dioxus::prelude::*;

/// Top navigation bar. Expects a `Route` enum that implements `Routable`.
/// We keep this generic with `children` so the web crate can pass `Link` elements.
#[component]
pub fn Navbar(
    children: Element,
    dark_mode: bool,
    on_toggle: EventHandler<()>,
) -> Element {
    let icon = if dark_mode { "☀" } else { "☾" };
    rsx! {
        nav {
            class: "app-nav",
            span { class: "nav-brand", "Budget" }
            {children}
            button {
                class: "nav-theme-btn",
                onclick: move |_| on_toggle.call(()),
                "{icon}"
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
