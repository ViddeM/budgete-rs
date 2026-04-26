use api::models::{Category, Transaction};
use dioxus::prelude::*;

use crate::category_badge::{CategoryBadge, UnprocessedBadge};
use crate::format::fmt_tx_amount;

/// Props for an optional classify / reclassify action rendered inside the row.
/// Pass `None` as the category to remove the classification.
#[derive(Clone, PartialEq)]
pub struct ClassifyAction {
    pub categories: Vec<Category>,
    pub on_classify: EventHandler<(Transaction, Option<Category>)>,
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

    let cat = transaction.category.clone();

    rsx! {
        div {
            style: "display: grid; grid-template-columns: 90px 1fr 72px 150px 100px; gap: 12px; align-items: center; padding: 10px 0; border-bottom: 1px solid #f3f4f6;",

            // Date
            span {
                style: "font-size: 0.8rem; color: #6b7280; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                "{date_str}"
            }

            // Description
            span {
                style: "font-size: 0.9rem; color: #111827; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                "{transaction.description}"
            }

            // Source badge
            span {
                style: "font-size: 0.7rem; color: #9ca3af; text-transform: uppercase; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                "{transaction.source}"
            }

            // Category badge / classify dropdown
            if let Some(ref action) = classify_action {
                // Show a select for classification / reclassification
                select {
                    style: "width: 100%; font-size: 0.8rem; border: 1px solid #d1d5db; border-radius: 6px; padding: 2px 4px; color: #374151; box-sizing: border-box;",
                    onchange: {
                        let tx = transaction.clone();
                        let cats = action.categories.clone();
                        let handler = action.on_classify.clone();
                        move |evt: Event<FormData>| {
                            let val = evt.value();
                            if val.is_empty() {
                                handler.call((tx.clone(), None));
                            } else if let Some(cat) = cats.iter().find(|c| c.id.to_string() == val) {
                                handler.call((tx.clone(), Some(cat.clone())));
                            }
                        }
                    },
                    option { value: "", selected: cat.is_none(), "— unclassify —" }
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
                style: "text-align: right; font-weight: 600; font-size: 0.9rem; color: {amount_color};",
                "{amount_str}"
            }
        }
    }
}
