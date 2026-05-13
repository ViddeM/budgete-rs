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
            p { class: "tx-empty", "No transactions." }
        };
    }

    rsx! {
        div {
            class: "tx-list",

            // Header row — must use the same grid template as TransactionRow
            div {
                class: "tx-list__header",
                span { "Date" }
                span { "Description" }
                span { "Source" }
                span { "Category" }
                span { class: "tx-list__header-amount", "Amount" }
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
