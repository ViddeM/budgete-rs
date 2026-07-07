use api::models::CreateTransactionRequest;
use dioxus::prelude::*;
use rust_decimal::Decimal;

/// Spreadsheet-like form for adding many transactions at once.
///
/// Each row exposes date, description, amount, currency and source. New rows
/// default to currency SEK and source "manual". Validation errors are shown
/// inline; only rows that pass validation are emitted for creation.
#[component]
pub fn BulkTransactionForm(
    on_submit: EventHandler<Vec<CreateTransactionRequest>>,
    loading: bool,
) -> Element {
    #[derive(Clone, Default)]
    struct Row {
        date: String,
        description: String,
        amount: String,
        currency: String,
        source: String,
    }

    let mut rows = use_signal(|| {
        vec![Row {
            currency: "SEK".to_string(),
            source: "manual".to_string(),
            ..Default::default()
        }]
    });
    let mut error: Signal<Option<String>> = use_signal(|| None);

    let add_row = move |_| {
        rows.write().push(Row {
            currency: "SEK".to_string(),
            source: "manual".to_string(),
            ..Default::default()
        });
    };

    let mut remove_row = move |idx: usize| {
        rows.write().remove(idx);
    };

    let submit = move |_| {
        error.set(None);

        let mut reqs = Vec::new();
        let mut first_error: Option<String> = None;

        for (idx, row) in rows().iter().enumerate() {
            // Skip completely empty rows.
            if row.description.trim().is_empty() && row.amount.trim().is_empty() {
                continue;
            }

            let description = row.description.trim().to_string();
            if description.is_empty() {
                first_error = Some(format!("Row {}: description is required", idx + 1));
                continue;
            }

            let amount = match row.amount.replace(',', ".").parse::<Decimal>() {
                Ok(v) if !v.is_zero() => v,
                Ok(_) => {
                    first_error = Some(format!("Row {}: amount cannot be zero", idx + 1));
                    continue;
                }
                Err(_) => {
                    first_error = Some(format!("Row {}: invalid amount", idx + 1));
                    continue;
                }
            };

            let date = if row.date.trim().is_empty() {
                None
            } else {
                match chrono::NaiveDate::parse_from_str(row.date.trim(), "%Y-%m-%d") {
                    Ok(d) => Some(d),
                    Err(_) => {
                        first_error = Some(format!(
                            "Row {}: invalid date format. Use YYYY-MM-DD",
                            idx + 1
                        ));
                        continue;
                    }
                }
            };

            let currency = row.currency.trim().to_ascii_uppercase().to_string();
            if currency.is_empty() {
                first_error = Some(format!("Row {}: currency is required", idx + 1));
                continue;
            }

            let source = row.source.trim().to_string();
            if source.is_empty() {
                first_error = Some(format!("Row {}: source is required", idx + 1));
                continue;
            }

            reqs.push(CreateTransactionRequest {
                date,
                description,
                amount,
                currency,
                source,
            });
        }

        if let Some(err) = first_error {
            error.set(Some(err));
            return;
        }

        if reqs.is_empty() {
            error.set(Some("Add at least one transaction".to_string()));
            return;
        }

        on_submit.call(reqs);
        rows.set(vec![Row {
            currency: "SEK".to_string(),
            source: "manual".to_string(),
            ..Default::default()
        }]);
    };

    rsx! {
        div {
            class: "form-card bulk-form",
            p { class: "form-card__title", "Add multiple transactions" }

            div {
                class: "bulk-form__header",
                span { "Date" }
                span { "Description" }
                span { "Amount" }
                span { "Currency" }
                span { "Source" }
                span {}
            }

            for (idx, _) in rows().iter().enumerate() {
                div {
                    key: "{idx}",
                    class: "bulk-form__row",
                    input {
                        r#type: "date",
                        class: "input-std",
                        disabled: loading,
                        value: rows()[idx].date.clone(),
                        oninput: move |e| rows.write()[idx].date = e.value(),
                    }
                    input {
                        r#type: "text",
                        class: "input-std",
                        disabled: loading,
                        value: rows()[idx].description.clone(),
                        placeholder: "Description",
                        oninput: move |e| rows.write()[idx].description = e.value(),
                    }
                    input {
                        r#type: "text",
                        inputmode: "decimal",
                        class: "input-std",
                        disabled: loading,
                        value: rows()[idx].amount.clone(),
                        placeholder: "-123.45",
                        oninput: move |e| rows.write()[idx].amount = e.value(),
                    }
                    input {
                        r#type: "text",
                        class: "input-std",
                        disabled: loading,
                        value: rows()[idx].currency.clone(),
                        oninput: move |e| rows.write()[idx].currency = e.value().to_ascii_uppercase(),
                    }
                    input {
                        r#type: "text",
                        class: "input-std",
                        disabled: loading,
                        value: rows()[idx].source.clone(),
                        oninput: move |e| rows.write()[idx].source = e.value(),
                    }
                    button {
                        class: "btn-ghost btn-ghost--sm",
                        disabled: loading,
                        onclick: move |_| remove_row(idx),
                        "Remove"
                    }
                }
            }

            if let Some(err) = error() {
                p { class: "form-error", style: "margin-top: 12px;", "{err}" }
            }

            div {
                class: "bulk-form__actions",
                button {
                    class: "btn-ghost",
                    disabled: loading,
                    onclick: add_row,
                    "+ Add row"
                }
                button {
                    class: "btn-primary",
                    disabled: loading,
                    onclick: submit,
                    "Add transactions"
                }
            }
        }
    }
}
