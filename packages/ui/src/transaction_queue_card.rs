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
    let amount_color = if amount >= rust_decimal::Decimal::ZERO { "#16a34a" } else { "#dc2626" };
    let amount_str = fmt_tx_amount(amount, &transaction.currency);

    let date_str = transaction
        .date
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "Pending".to_string());

    // Separate into top-level and subcategories.
    let parents: Vec<&Category> =
        categories.iter().filter(|c| c.parent_id.is_none()).collect();
    let has_any_subcats = categories.iter().any(|c| c.parent_id.is_some());

    rsx! {
        div {
            style: "background: #fff; border: 1px solid #e5e7eb; border-radius: 16px; padding: 28px 32px; max-width: 560px;",

            // Meta line: date · source
            p {
                style: "font-size: 0.78rem; color: #9ca3af; margin: 0 0 6px; text-transform: uppercase; letter-spacing: 0.05em;",
                "{date_str} · {transaction.source}"
            }

            // Description
            p {
                style: "font-size: 1.25rem; font-weight: 600; color: #111827; margin: 0 0 10px; line-height: 1.4;",
                "{transaction.description}"
            }

            // Amount
            p {
                style: "font-size: 2rem; font-weight: 700; color: {amount_color}; margin: 0 0 28px;",
                "{amount_str}"
            }

            // Category picker — subcategories only, grouped under parents
            if !has_any_subcats {
                p {
                    style: "font-size: 0.85rem; color: #9ca3af;",
                    "Add subcategories to begin classifying."
                }
            } else {
                p {
                    style: "font-size: 0.75rem; font-weight: 700; color: #6b7280; margin: 0 0 12px; text-transform: uppercase; letter-spacing: 0.05em;",
                    "Assign category"
                }
                div {
                    style: "display: flex; flex-direction: column; gap: 12px;",
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
                                        // Parent label
                                        p {
                                            style: "font-size: 0.72rem; font-weight: 700; color: #9ca3af; margin: 0 0 6px; text-transform: uppercase; letter-spacing: 0.05em;",
                                            "{parent.name}"
                                        }
                                        // Subcategory buttons
                                        div {
                                            style: "display: flex; flex-wrap: wrap; gap: 8px;",
                                            for sub in subcats.iter() {
                                                CategoryButton {
                                                    key: "{sub.id}",
                                                    category: (*sub).clone(),
                                                    transaction: transaction.clone(),
                                                    on_classify: on_classify.clone(),
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
    let filter = if hovered() { hover_filter(&category.color) } else { "none" };
    rsx! {
        button {
            style: "background: {category.color}; color: {text_color}; border: none; padding: 8px 20px; border-radius: 999px; font-size: 0.9rem; font-weight: 600; cursor: pointer; transition: filter 0.15s ease; filter: {filter};",
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
