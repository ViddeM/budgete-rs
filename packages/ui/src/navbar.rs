use dioxus::prelude::*;

/// Top navigation bar. Expects a `Route` enum that implements `Routable`.
/// We keep this generic with `children` so the web crate can pass `Link` elements.
///
/// HTML shape:
/// ```text
/// nav.app-nav                 ← block element
///   div.nav-bar               ← flex row: brand | links | controls
///     span.nav-brand
///     div.nav-links           ← desktop: visible flex row; mobile: hidden
///       {children}
///     div.nav-controls
///   div.nav-drawer            ← mobile drawer: block below nav-bar, hidden by default
///     {children}              ← same links, also rendered here for mobile
/// ```
///
/// Because `Element` is not `Clone` in Dioxus 0.7, the links are passed via a
/// render prop closure so they can be called twice — once for the desktop row
/// and once for the mobile drawer.
#[component]
pub fn Navbar(
    #[props(into)] render_links: Callback<(), Element>,
    dark_mode: bool,
    on_toggle: EventHandler<()>,
) -> Element {
    let mut nav_open = use_signal(|| false);
    let icon = if dark_mode { "☀" } else { "☾" };
    rsx! {
        nav { class: "app-nav",
            div { class: "nav-bar",
                span { class: "nav-brand", "Budget" }
                div { class: "nav-links", {render_links.call(())} }
                div { class: "nav-controls",
                    button {
                        class: "nav-theme-btn",
                        onclick: move |_| on_toggle.call(()),
                        "{icon}"
                    }
                    button {
                        class: "nav-hamburger",
                        onclick: move |_| *nav_open.write() = !nav_open(),
                        if nav_open() { "✕" } else { "☰" }
                    }
                }
            }
            div {
                class: if nav_open() { "nav-drawer nav-drawer--open" } else { "nav-drawer" },
                // Any tap on a link inside the drawer bubbles up here and closes the menu.
                onclick: move |_| *nav_open.write() = false,
                {render_links.call(())}
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
