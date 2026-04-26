use api::models::Category;
use dioxus::prelude::*;

use crate::format::{contrast_text, hover_filter};

/// A small colored pill showing a category name.
#[component]
pub fn CategoryBadge(category: Category) -> Element {
    let mut hovered = use_signal(|| false);
    let text_color = contrast_text(&category.color);
    let filter = if hovered() { hover_filter(&category.color) } else { "none" };
    rsx! {
        span {
            style: "background-color: {category.color}; color: {text_color}; padding: 2px 10px; border-radius: 999px; font-size: 0.75rem; font-weight: 600; cursor: default; transition: filter 0.15s ease; filter: {filter};",
            onmouseenter: move |_| hovered.set(true),
            onmouseleave: move |_| hovered.set(false),
            "{category.name}"
        }
    }
}

/// A faint grey badge for unprocessed transactions.
#[component]
pub fn UnprocessedBadge() -> Element {
    rsx! {
        span {
            style: "background-color: #e5e7eb; color: #6b7280; padding: 2px 10px; border-radius: 999px; font-size: 0.75rem; font-weight: 600;",
            "Unprocessed"
        }
    }
}
