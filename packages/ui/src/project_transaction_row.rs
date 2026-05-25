use api::models::Transaction;
use dioxus::prelude::*;
use uuid::Uuid;

use crate::format::{fmt_date, fmt_tx_amount, tx_amount_color};

/// A compact transaction row used inside the Projects view.
///
/// Renders the date, description, optional category name, and amount of a
/// transaction alongside a single action button (e.g. "Remove" or "Add").
/// The caller provides the button label and an event handler that receives the
/// transaction's [`Uuid`] when the button is clicked.
#[component]
pub fn ProjectTransactionRow(
    transaction: Transaction,
    action_label: String,
    on_action: EventHandler<Uuid>,
) -> Element {
    let tid = transaction.id;
    let date_str = fmt_date(transaction.date);
    let amount_str = fmt_tx_amount(transaction.amount, &transaction.currency);
    let amount_color = tx_amount_color(transaction.amount);
    let desc = transaction.description.clone();
    let cat_name = transaction.category.as_ref().map(|c| c.name.clone());

    rsx! {
        div {
            key: "{tid}",
            class: "project-tx-row",
            span { class: "project-row__date", "{date_str}" }
            span {
                class: "project-row__desc",
                style: "font-size: 0.88rem; color: var(--text-secondary);",
                "{desc}"
            }
            if let Some(cat) = cat_name {
                span { class: "project-row__cat", "{cat}" }
            }
            span {
                class: "project-row__amount",
                style: "font-size: 0.88rem; color: {amount_color};",
                "{amount_str}"
            }
            button {
                onclick: move |_| on_action.call(tid),
                class: "btn-ghost btn-ghost--sm",
                "{action_label}"
            }
        }
    }
}
