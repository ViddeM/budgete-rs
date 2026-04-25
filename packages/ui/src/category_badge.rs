use api::models::Category;
use dioxus::prelude::*;

/// A small colored pill showing a category name.
#[component]
pub fn CategoryBadge(category: Category) -> Element {
    rsx! {
        span {
            style: "background-color: {category.color}; color: #fff; padding: 2px 10px; border-radius: 999px; font-size: 0.75rem; font-weight: 600;",
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
