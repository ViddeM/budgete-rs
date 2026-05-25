use api::models::{Category, Transaction};
use dioxus::prelude::*;

use crate::format::{contrast_text, fmt_tx_amount, hover_filter};

/// A prominent card that presents a single transaction for classification.
///
/// Only subcategories (those with `parent_id.is_some()`) are shown as
/// selectable buttons; top-level categories appear as non-clickable group
/// headers. Parents that have no subcategories are omitted.
#[component]
pub fn TransactionQueueCard(
    transaction: Transaction,
    categories: Vec<Category>,
    on_classify: EventHandler<(Transaction, Category)>,
) -> Element {
    let amount = transaction.amount;
    let amount_color = if amount >= rust_decimal::Decimal::ZERO {
        "#16a34a"
    } else {
        "#dc2626"
    };
    let amount_str = fmt_tx_amount(amount, &transaction.currency);

    let date_str = transaction
        .date
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "Pending".to_string());

    // Separate into top-level and subcategories.
    let parents: Vec<&Category> = categories
        .iter()
        .filter(|c| c.parent_id.is_none())
        .collect();
    let has_any_subcats = categories.iter().any(|c| c.parent_id.is_some());

    rsx! {
        div {
            class: "queue-card",

            p { class: "queue-card__meta", "{date_str} · {transaction.source}" }
            p { class: "queue-card__desc", "{transaction.description}" }
            p { class: "queue-card__amount", style: "color: {amount_color};", "{amount_str}" }

            if !has_any_subcats {
                p { class: "queue-card__no-cats", "Add subcategories to begin classifying." }
            } else {
                p { class: "queue-card__pick-label", "Assign category" }
                div {
                    class: "queue-card__groups",
                    for parent in parents.iter() {
                        {
                            let parent_id = parent.id;
                            let subcats: Vec<&Category> = categories
                                .iter()
                                .filter(|c| c.parent_id == Some(parent_id))
                                .collect();
                            if subcats.is_empty() {
                                rsx! {}
                            } else {
                                rsx! {
                                    div {
                                        p { class: "queue-card__group-label", "{parent.name}" }
                                        div {
                                            class: "queue-card__subcats",
                                            for sub in subcats.iter() {
                                                CategoryButton {
                                                    key: "{sub.id}",
                                                    category: (*sub).clone(),
                                                    transaction: transaction.clone(),
                                                    on_classify,
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// A single subcategory button with adaptive text color and hover effect.
#[component]
fn CategoryButton(
    category: Category,
    transaction: Transaction,
    on_classify: EventHandler<(Transaction, Category)>,
) -> Element {
    let mut hovered = use_signal(|| false);
    let text_color = contrast_text(&category.color);
    let filter = if hovered() {
        hover_filter(&category.color)
    } else {
        "none"
    };
    rsx! {
        button {
            class: "cat-btn",
            style: "background: {category.color}; color: {text_color}; filter: {filter};",
            onmouseenter: move |_| hovered.set(true),
            onmouseleave: move |_| hovered.set(false),
            onclick: {
                let tx = transaction.clone();
                let cat = category.clone();
                move |_| on_classify.call((tx.clone(), cat.clone()))
            },
            "{category.name}"
        }
    }
}
