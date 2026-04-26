use api::models::{Category, Transaction, TransactionFilter};
use api::{classify_transaction, get_transactions, list_categories};
use dioxus::prelude::*;
use ui::{ClassifyAction, TransactionList};

#[component]
pub fn Transactions() -> Element {
    let mut transactions_res = use_resource(|| async {
        get_transactions(TransactionFilter::default()).await
    });
    let categories_res = use_resource(list_categories);

    let categories: Vec<Category> =
        categories_res().and_then(|r| r.ok()).unwrap_or_default();

    let on_classify = move |(tx, cat): (Transaction, Option<Category>)| async move {
        let _ = classify_transaction(tx.id, cat.map(|c| c.id)).await;
        transactions_res.restart();
    };

    rsx! {
        div {
            style: "padding: 32px; font-family: sans-serif; max-width: 960px;",
            h1 { style: "margin: 0 0 24px; font-size: 1.5rem; color: #111827;", "Transactions" }

            match transactions_res() {
                None => rsx! { p { style: "color: #6b7280;", "Loading…" } },
                Some(Err(e)) => rsx! { p { style: "color: #dc2626;", "Error: {e}" } },
                Some(Ok(all_txs)) => {
                    let txs: Vec<Transaction> = all_txs
                        .into_iter()
                        .filter(|tx| !tx.is_pending)
                        .collect();
                    rsx! {
                        TransactionList {
                            transactions: txs,
                            classify_action: Some(ClassifyAction {
                                categories: categories.clone(),
                                on_classify: EventHandler::new(on_classify),
                            }),
                        }
                    }
                }
            }
        }
    }
}
