use api::models::CreateTransactionRequest;
use dioxus::prelude::*;
use rust_decimal::Decimal;

/// Form for entering a single transaction manually.
///
/// Submits a `CreateTransactionRequest` so the caller can persist it. The
/// transaction always enters the unprocessed queue; there is no category
/// selector.
#[component]
pub fn ManualTransactionForm(
    on_submit: EventHandler<CreateTransactionRequest>,
    loading: bool,
) -> Element {
    #[derive(Clone)]
    struct FormState {
        date: String,
        description: String,
        amount: String,
        currency: String,
        source: String,
    }

    let initial = FormState {
        date: String::new(),
        description: String::new(),
        amount: String::new(),
        currency: "SEK".to_string(),
        source: "manual".to_string(),
    };

    let mut state = use_signal(|| initial.clone());
    let mut error: Signal<Option<String>> = use_signal(|| None);

    let submit = move |_| {
        error.set(None);

        let parsed_amount = match state().amount.replace(',', ".").parse::<Decimal>() {
            Ok(v) if !v.is_zero() => v,
            Ok(_) => {
                error.set(Some("Amount cannot be zero".to_string()));
                return;
            }
            Err(_) => {
                error.set(Some("Invalid amount".to_string()));
                return;
            }
        };

        let date = if state().date.trim().is_empty() {
            None
        } else {
            match chrono::NaiveDate::parse_from_str(state().date.trim(), "%Y-%m-%d") {
                Ok(d) => Some(d),
                Err(_) => {
                    error.set(Some("Invalid date format. Use YYYY-MM-DD".to_string()));
                    return;
                }
            }
        };

        let description = state().description.trim().to_string();
        if description.is_empty() {
            error.set(Some("Description cannot be empty".to_string()));
            return;
        }

        let source = state().source.trim().to_string();
        if source.is_empty() {
            error.set(Some("Source cannot be empty".to_string()));
            return;
        }

        let currency = state().currency.trim().to_ascii_uppercase().to_string();
        if currency.is_empty() {
            error.set(Some("Currency cannot be empty".to_string()));
            return;
        }

        on_submit.call(CreateTransactionRequest {
            date,
            description,
            amount: parsed_amount,
            currency,
            source,
        });

        // Reset after successful submit.
        state.set(FormState {
            date: String::new(),
            description: String::new(),
            amount: String::new(),
            currency: "SEK".to_string(),
            source: "manual".to_string(),
        });
    };

    rsx! {
        div {
            class: "form-card",
            div {
                class: "form-field",
                label { class: "form-label", "Date" }
                input {
                    r#type: "date",
                    class: "input-std input-std--full",
                    disabled: loading,
                    value: state().date,
                    oninput: move |e| { state.write().date = e.value(); },
                }
                p { class: "field-hint", "Leave blank to create a pending transaction." }
            }
            div {
                class: "form-field",
                style: "margin-top: 12px;",
                label { class: "form-label", "Description" }
                input {
                    r#type: "text",
                    class: "input-std input-std--full",
                    disabled: loading,
                    value: state().description,
                    placeholder: "e.g. Grocery store",
                    oninput: move |e| { state.write().description = e.value(); },
                }
            }
            div {
                class: "form-field",
                style: "margin-top: 12px;",
                label { class: "form-label", "Amount" }
                input {
                    r#type: "text",
                    inputmode: "decimal",
                    class: "input-std input-std--full",
                    disabled: loading,
                    value: state().amount,
                    placeholder: "e.g. -123.45",
                    oninput: move |e| { state.write().amount = e.value(); },
                }
                p { class: "field-hint", "Negative for expenses, positive for income." }
            }
            div {
                class: "form-row",
                style: "margin-top: 12px;",
                div {
                    class: "form-field",
                    style: "flex: 1;",
                    label { class: "form-label", "Currency" }
                    input {
                        r#type: "text",
                        class: "input-std input-std--full",
                        disabled: loading,
                        value: state().currency,
                        oninput: move |e| { state.write().currency = e.value().to_ascii_uppercase(); },
                    }
                }
                div {
                    class: "form-field",
                    style: "flex: 1;",
                    label { class: "form-label", "Source" }
                    input {
                        r#type: "text",
                        class: "input-std input-std--full",
                        disabled: loading,
                        value: state().source,
                        oninput: move |e| { state.write().source = e.value(); },
                    }
                }
            }
            if let Some(err) = error() {
                p { class: "form-error", style: "margin-top: 12px;", "{err}" }
            }
            button {
                class: "btn-primary",
                style: "margin-top: 16px;",
                disabled: loading,
                onclick: submit,
                "Add transaction"
            }
        }
    }
}
