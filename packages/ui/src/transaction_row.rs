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
            class: "tx-row",

            // Date
            span { class: "tx-row__date", "{date_str}" }

            // Description
            span { class: "tx-row__desc", "{transaction.description}" }

            // Source badge
            span { class: "tx-row__source", "{transaction.source}" }

            // Category badge / classify dropdown
            if let Some(ref action) = classify_action {
                select {
                    class: "tx-row__classify",
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
                if let Some(c) = cat {
                    CategoryBadge { category: c }
                } else {
                    UnprocessedBadge {}
                }
            }

            // Amount
            span {
                class: "tx-row__amount",
                style: "color: {amount_color};",
                "{amount_str}"
            }
        }
    }
}
