use dioxus::prelude::*;

/// A single summary card showing a label, a primary value, and an optional sub-label.
/// `value_color` defaults to `var(--text-primary)` if not provided.
#[component]
pub fn StatCard(
    label: String,
    value: String,
    sub_label: Option<String>,
    #[props(default = "var(--text-primary)".to_string())]
    value_color: String,
) -> Element {
    rsx! {
        div {
            style: "background: var(--bg-card); border: 1px solid var(--border); border-radius: 12px; padding: 20px 24px; min-width: 160px;",
            p {
                style: "margin: 0 0 4px; font-size: 0.8rem; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.05em;",
                "{label}"
            }
            p {
                style: "margin: 0; font-size: 1.6rem; font-weight: 700; color: {value_color};",
                "{value}"
            }
            if let Some(sub) = sub_label {
                p {
                    style: "margin: 4px 0 0; font-size: 0.78rem; color: var(--text-muted);",
                    "{sub}"
                }
            }
        }
    }
}
