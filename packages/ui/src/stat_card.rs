use dioxus::prelude::*;

/// A single summary card showing a label, a primary value, and an optional sub-label.
/// `value_color` defaults to `var(--text-primary)` if not provided.
#[component]
pub fn StatCard(
    label: String,
    value: String,
    sub_label: Option<String>,
    #[props(default = "var(--text-primary)".to_string())] value_color: String,
) -> Element {
    rsx! {
        div {
            class: "stat-card",
            p { class: "stat-card__label", "{label}" }
            p { class: "stat-card__value", style: "color: {value_color};", "{value}" }
            if let Some(sub) = sub_label {
                p { class: "stat-card__sub", "{sub}" }
            }
        }
    }
}
