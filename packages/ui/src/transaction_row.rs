use api::models::{Category, Transaction};
use dioxus::prelude::*;

use crate::category_badge::{CategoryBadge, UnprocessedBadge};
use crate::format::fmt_tx_amount;

/// Props for an optional classify action rendered inside the row.
#[derive(Clone, PartialEq)]
pub struct ClassifyAction {
    pub categories: Vec<Category>,
    pub on_classify: EventHandler<(Transaction, Category)>,
}

/// A single transaction row.
#[component]
pub fn TransactionRow(
    transaction: Transaction,
    classify_action: Option<ClassifyAction>,
) -> Element {
    let amount = transaction.amount;
    let amount_color = if amount >= rust_decimal::Decimal::ZERO { "#16a34a" } else { "#dc2626" };
    let amount_str = fmt_tx_amount(amount, &transaction.currency);

    let date_str = transaction
        .date
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "Pending".to_string());

    let cat = transaction
        .category_id
        .zip(transaction.category_name.clone())
        .zip(transaction.category_color.clone())
        .map(|((id, name), color)| Category { id, name, color, parent_id: None });

    rsx! {
        div {
            style: "display: flex; align-items: center; gap: 12px; padding: 10px 0; border-bottom: 1px solid #f3f4f6;",

            // Date
            span {
                style: "min-width: 90px; font-size: 0.8rem; color: #6b7280;",
                "{date_str}"
            }

            // Description
            span {
                style: "flex: 1; font-size: 0.9rem; color: #111827; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                "{transaction.description}"
            }

            // Source badge
            span {
                style: "font-size: 0.7rem; color: #9ca3af; text-transform: uppercase;",
                "{transaction.source}"
            }

            // Category badge / classify dropdown
            if let Some(ref action) = classify_action {
                // Show a select for classification
                select {
                    style: "font-size: 0.8rem; border: 1px solid #d1d5db; border-radius: 6px; padding: 2px 6px; color: #374151;",
                    onchange: {
                        let tx = transaction.clone();
                        let cats = action.categories.clone();
                        let handler = action.on_classify.clone();
                        move |evt: Event<FormData>| {
                            let val = evt.value();
                            if let Some(cat) = cats.iter().find(|c| c.id.to_string() == val) {
                                handler.call((tx.clone(), cat.clone()));
                            }
                        }
                    },
                    option { value: "", disabled: true, selected: cat.is_none(), "— assign —" }
                    for c in action.categories.iter() {
                        option {
                            value: "{c.id}",
                            selected: cat.as_ref().map(|x| x.id) == Some(c.id),
                            "{c.name}"
                        }
                    }
                }
            } else {
                // Read-only badge
                if let Some(c) = cat {
                    CategoryBadge { category: c }
                } else {
                    UnprocessedBadge {}
                }
            }

            // Amount
            span {
                style: "min-width: 90px; text-align: right; font-weight: 600; font-size: 0.9rem; color: {amount_color};",
                "{amount_str}"
            }
        }
    }
}
