use api::models::{Category, Transaction, TransactionFilter, UpdateTransactionRequest};
use api::{classify_transaction, delete_transaction, get_transactions, list_categories,
          update_transaction};
use dioxus::prelude::*;
use ui::{ClassifyAction, TransactionList, TxMenuAction};
use uuid::Uuid;

#[component]
pub fn Transactions() -> Element {
    let mut transactions_res =
        use_resource(|| async { get_transactions(TransactionFilter::default()).await });
    let categories_res = use_resource(list_categories);

    let categories: Vec<Category> = categories_res().and_then(|r| r.ok()).unwrap_or_default();

    let on_classify = move |(tx, cat): (Transaction, Option<Category>)| async move {
        let _ = classify_transaction(tx.id, cat.map(|c| c.id)).await;
        transactions_res.restart();
    };

    let on_edit = move |req: UpdateTransactionRequest| async move {
        let _ = update_transaction(req).await;
        transactions_res.restart();
    };

    let on_delete = move |id: Uuid| async move {
        let _ = delete_transaction(id).await;
        transactions_res.restart();
    };

    rsx! {
        div {
            class: "view view--wide",
            h1 { class: "view__title", "Transactions" }

            match transactions_res() {
                None => rsx! { p { style: "color: var(--text-muted);", "Loading\u{2026}" } },
                Some(Err(e)) => rsx! { p { class: "text-error", "Error: {e}" } },
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
                            menu_action: Some(TxMenuAction {
                                on_edit: EventHandler::new(on_edit),
                                on_delete: EventHandler::new(on_delete),
                            }),
                        }
                    }
                }
            }
        }
    }
}
