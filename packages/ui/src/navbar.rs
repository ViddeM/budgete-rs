use dioxus::prelude::*;

/// Top navigation bar. Expects a `Route` enum that implements `Routable`.
/// We keep this generic with `children` so the web crate can pass `Link` elements.
#[component]
pub fn Navbar(children: Element) -> Element {
    rsx! {
        nav {
            style: "display: flex; align-items: center; gap: 24px; padding: 0 32px; height: 52px; background: #1e293b; color: #f1f5f9;",
            span {
                style: "font-weight: 700; font-size: 1.1rem; color: #f1f5f9; margin-right: 16px;",
                "Budget"
            }
            {children}
        }
    }
}

/// A single nav link styled for the dark navbar.
#[component]
pub fn NavLink(children: Element) -> Element {
    rsx! {
        span {
            style: "font-size: 0.9rem; color: #94a3b8;",
            {children}
        }
    }
}
