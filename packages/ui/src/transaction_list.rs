use api::models::Transaction;
use dioxus::prelude::*;

use crate::transaction_row::{ClassifyAction, TransactionRow};

/// A scrollable list of transactions.
#[component]
pub fn TransactionList(
    transactions: Vec<Transaction>,
    /// If provided, each row will show a classification dropdown.
    classify_action: Option<ClassifyAction>,
) -> Element {
    if transactions.is_empty() {
        return rsx! {
            p { style: "color: #6b7280; text-align: center; padding: 24px 0;", "No transactions." }
        };
    }

    rsx! {
        div {
            style: "font-family: sans-serif;",

            // Header row
            div {
                style: "display: flex; gap: 12px; padding: 6px 0; border-bottom: 2px solid #e5e7eb; font-size: 0.75rem; font-weight: 700; color: #9ca3af; text-transform: uppercase;",
                span { style: "min-width: 90px;", "Date" }
                span { style: "flex: 1;", "Description" }
                span { style: "font-size: 0.7rem;", "Source" }
                span { "Category" }
                span { style: "min-width: 90px; text-align: right;", "Amount" }
            }

            for tx in transactions.iter() {
                TransactionRow {
                    key: "{tx.id}",
                    transaction: tx.clone(),
                    classify_action: classify_action.clone(),
                }
            }
        }
    }
}
