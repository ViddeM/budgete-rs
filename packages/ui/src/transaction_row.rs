use api::models::{Category, Transaction, UpdateTransactionRequest};
use dioxus::prelude::*;
use uuid::Uuid;

use crate::category_badge::{CategoryBadge, UnprocessedBadge};
use crate::format::{fmt_date, fmt_tx_amount, tx_amount_color};

/// Props for an optional classify / reclassify action rendered inside the row.
/// Pass `None` as the category to remove the classification.
#[derive(Clone, PartialEq)]
pub struct ClassifyAction {
    pub categories: Vec<Category>,
    pub on_classify: EventHandler<(Transaction, Option<Category>)>,
}

/// Props for edit / delete actions available via the kebab menu.
#[derive(Clone, PartialEq)]
pub struct TxMenuAction {
    pub on_edit: EventHandler<UpdateTransactionRequest>,
    pub on_delete: EventHandler<Uuid>,
}

// ── Edit modal ────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct EditForm {
    date: String,
    description: String,
    amount: String,
    currency: String,
    source: String,
    error: String,
}

impl EditForm {
    fn from_tx(tx: &Transaction) -> Self {
        Self {
            date: tx.date.map(|d| d.to_string()).unwrap_or_default(),
            description: tx.description.clone(),
            amount: tx.amount.to_string(),
            currency: tx.currency.clone(),
            source: tx.source.clone(),
            error: String::new(),
        }
    }
}

#[component]
fn EditModal(
    transaction: Transaction,
    on_save: EventHandler<UpdateTransactionRequest>,
    on_cancel: EventHandler<()>,
) -> Element {
    let mut form = use_signal(|| EditForm::from_tx(&transaction));

    let tx_id = transaction.id;

    rsx! {
        // Backdrop
        div {
            class: "tx-modal-backdrop",
            onclick: move |_| on_cancel.call(()),

            div {
                class: "tx-modal",
                // Stop clicks inside the modal from closing it
                onclick: move |e| e.stop_propagation(),

                h3 { class: "tx-modal__title", "Edit Transaction" }

                div { class: "form-field",
                    label { class: "form-label", "Date" }
                    input {
                        class: "input-std input-std--full",
                        r#type: "date",
                        value: form.read().date.clone(),
                        oninput: move |e| form.write().date = e.value(),
                    }
                }

                div { class: "form-field",
                    label { class: "form-label", "Description" }
                    input {
                        class: "input-std input-std--full",
                        r#type: "text",
                        value: form.read().description.clone(),
                        oninput: move |e| form.write().description = e.value(),
                    }
                }

                div { class: "form-row",
                    div { class: "form-field",
                        label { class: "form-label", "Amount" }
                        input {
                            class: "input-std",
                            r#type: "text",
                            value: form.read().amount.clone(),
                            oninput: move |e| form.write().amount = e.value(),
                        }
                    }
                    div { class: "form-field",
                        label { class: "form-label", "Currency" }
                        input {
                            class: "input-std",
                            r#type: "text",
                            value: form.read().currency.clone(),
                            oninput: move |e| form.write().currency = e.value(),
                        }
                    }
                }

                div { class: "form-field",
                    label { class: "form-label", "Source" }
                    input {
                        class: "input-std input-std--full",
                        r#type: "text",
                        value: form.read().source.clone(),
                        oninput: move |e| form.write().source = e.value(),
                    }
                }

                if !form.read().error.is_empty() {
                    p { class: "form-error", "{form.read().error}" }
                }

                div { class: "tx-modal__actions",
                    button {
                        class: "btn-ghost",
                        onclick: move |_| on_cancel.call(()),
                        "Cancel"
                    }
                    button {
                        class: "btn-primary",
                        onclick: move |_| {
                            let f = form.read().clone();
                            // Parse amount — accept both comma and dot decimals
                            let amount_str = f.amount.replace(',', ".");
                            match amount_str.parse::<rust_decimal::Decimal>() {
                                Err(_) => form.write().error = "Amount must be a number".to_string(),
                                Ok(amount) => {
                                    let date = if f.date.is_empty() {
                                        None
                                    } else {
                                        match chrono::NaiveDate::parse_from_str(&f.date, "%Y-%m-%d") {
                                            Ok(d) => Some(d),
                                            Err(_) => {
                                                form.write().error =
                                                    "Date must be YYYY-MM-DD".to_string();
                                                return;
                                            }
                                        }
                                    };
                                    on_save.call(UpdateTransactionRequest {
                                        id: tx_id,
                                        date,
                                        description: Some(f.description),
                                        amount: Some(amount),
                                        currency: Some(f.currency),
                                        source: Some(f.source),
                                    });
                                }
                            }
                        },
                        "Save"
                    }
                }
            }
        }
    }
}

// ── TransactionRow ────────────────────────────────────────────────────────────

/// A single transaction row.
#[component]
pub fn TransactionRow(
    transaction: Transaction,
    classify_action: Option<ClassifyAction>,
    menu_action: Option<TxMenuAction>,
) -> Element {
    let amount = transaction.amount;
    let amount_color = tx_amount_color(amount);
    let amount_str = fmt_tx_amount(amount, &transaction.currency);

    let date_str = fmt_date(transaction.date);

    let cat = transaction.category.clone();

    // Kebab menu open state
    let mut menu_open = use_signal(|| false);
    // Edit modal open state
    let mut editing = use_signal(|| false);

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
                        let handler = action.on_classify;
                        move |evt: Event<FormData>| {
                            let val = evt.value();
                            if val.is_empty() {
                                handler.call((tx.clone(), None));
                            } else if let Some(cat) =
                                cats.iter().find(|c| c.id.to_string() == val)
                            {
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

            // Amount + optional kebab menu
            div { class: "tx-row__amount-cell",
                span {
                    class: "tx-row__amount",
                    style: "color: {amount_color};",
                    "{amount_str}"
                }

                if let Some(ref action) = menu_action {
                    div { class: "tx-kebab",
                        button {
                            class: "tx-kebab__btn",
                            r#type: "button",
                            title: "More actions",
                            onclick: move |e| {
                                e.stop_propagation();
                                let was_open = *menu_open.read();
                                *menu_open.write() = !was_open;
                            },
                            "\u{22EF}"
                        }

                        if *menu_open.read() {
                            // Click-outside layer
                            div {
                                class: "tx-kebab__backdrop",
                                onclick: move |_| *menu_open.write() = false,
                            }
                            div { class: "tx-kebab__menu",
                                button {
                                    class: "tx-kebab__item",
                                    r#type: "button",
                                    onclick: {
                                        move |_| {
                                            *menu_open.write() = false;
                                            *editing.write() = true;
                                        }
                                    },
                                    "Edit"
                                }
                                button {
                                    class: "tx-kebab__item tx-kebab__item--danger",
                                    r#type: "button",
                                    onclick: {
                                        let tx_id = transaction.id;
                                        let handler = action.on_delete;
                                        move |_| {
                                            *menu_open.write() = false;
                                            handler.call(tx_id);
                                        }
                                    },
                                    "Delete"
                                }
                            }
                        }
                    }
                }
            }

            // Edit modal (rendered outside the grid cell so it overlays everything)
            if *editing.read() {
                if let Some(ref action) = menu_action {
                    EditModal {
                        transaction: transaction.clone(),
                        on_save: {
                            let handler = action.on_edit;
                            move |req| {
                                *editing.write() = false;
                                handler.call(req);
                            }
                        },
                        on_cancel: move |_| *editing.write() = false,
                    }
                }
            }
        }
    }
}
