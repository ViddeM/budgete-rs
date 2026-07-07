use api::models::CreateTransactionRequest;
use dioxus::prelude::*;
use rust_decimal::Decimal;

/// Bulk-enter transactions by pasting CSV-like text.
///
/// Supported columns (detected in any order via header row): `date`,
/// `description`, `amount`, `currency`, `source`. If headers are absent, the
/// default column order is date, description, amount, (optionally currency and
/// source).
#[component]
pub fn BulkTransactionPaste(
    on_submit: EventHandler<Vec<CreateTransactionRequest>>,
    loading: bool,
) -> Element {
    let mut text = use_signal(String::new);
    let mut delimiter = use_signal(|| ",".to_string());
    let mut has_header = use_signal(|| false);
    let mut date_format = use_signal(|| "%Y-%m-%d".to_string());
    let mut preview: Signal<Option<Vec<(CreateTransactionRequest, bool)>>> = use_signal(|| None);
    let mut error: Signal<Option<String>> = use_signal(|| None);

    let delimiter_char = move || match delimiter().as_str() {
        "tab" => '\t',
        "semicolon" => ';',
        _ => ',',
    };

    let parse = move || -> Result<Vec<CreateTransactionRequest>, String> {
        let sep = delimiter_char();
        let raw = text();
        if raw.trim().is_empty() {
            return Err("Paste some transactions first".to_string());
        }

        let lines: Vec<&str> = raw
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();
        if lines.is_empty() {
            return Err("Paste some transactions first".to_string());
        }

        let mut start_idx = 0;
        let mut col_map = default_column_map();

        if has_header() {
            let header = split_line(lines[0], sep);
            col_map = build_column_map(&header)?;
            start_idx = 1;
        }

        let mut reqs = Vec::new();
        for (i, line) in lines.iter().enumerate().skip(start_idx) {
            let row = parse_line(line, sep, &col_map, &date_format())?;
            reqs.push((row, i + 1));
        }

        // Convert line numbers are no longer needed after parsing.
        Ok(reqs.into_iter().map(|(r, _)| r).collect())
    };

    let preview_clicked = move |_| {
        error.set(None);
        match parse() {
            Ok(reqs) => {
                preview.set(Some(reqs.into_iter().map(|r| (r, true)).collect()));
            }
            Err(e) => {
                preview.set(None);
                error.set(Some(e));
            }
        }
    };

    let submit = move |_| {
        error.set(None);
        match parse() {
            Ok(reqs) => {
                if reqs.is_empty() {
                    error.set(Some("No transactions to add".to_string()));
                    return;
                }
                on_submit.call(reqs);
                text.set(String::new());
                preview.set(None);
            }
            Err(e) => error.set(Some(e)),
        }
    };

    rsx! {
        div {
            class: "form-card bulk-paste",
            p { class: "form-card__title", "Paste transactions" }

            div {
                class: "form-row bulk-paste__options",
                div {
                    class: "form-field",
                    label { class: "form-label", "Delimiter" }
                    select {
                        class: "input-std",
                        disabled: loading,
                        onchange: move |e| delimiter.set(e.value()),
                        option { value: ",", "Comma" }
                        option { value: "semicolon", "Semicolon" }
                        option { value: "tab", "Tab" }
                    }
                }
                div {
                    class: "form-field",
                    label { class: "form-label", "Date format" }
                    select {
                        class: "input-std",
                        disabled: loading,
                        value: date_format(),
                        onchange: move |e| date_format.set(e.value()),
                        option { value: "%Y-%m-%d", "YYYY-MM-DD" }
                        option { value: "%d/%m/%Y", "DD/MM/YYYY" }
                        option { value: "%m/%d/%Y", "MM/DD/YYYY" }
                        option { value: "%Y/%m/%d", "YYYY/MM/DD" }
                    }
                }
                label {
                    class: "checkbox-row",
                    input {
                        r#type: "checkbox",
                        disabled: loading,
                        checked: has_header(),
                        oninput: move |e| has_header.set(e.checked()),
                    }
                    span { "First row is header" }
                }
            }

            textarea {
                class: "input-std input-std--full bulk-paste__textarea",
                rows: "8",
                disabled: loading,
                value: text(),
                placeholder: "2026-07-01, Grocery store, -500.00\n2026-07-02, Salary, 25000.00",
                oninput: move |e| { text.set(e.value()); preview.set(None); },
            }
            p { class: "field-hint",
                if has_header() {
                    "Header columns: date, description, amount, currency (optional), source (optional)."
                } else {
                    "Columns: date, description, amount, currency (optional), source (optional). Leave date blank for pending."
                }
            }

            if let Some(p) = preview() {
                div {
                    class: "bulk-paste__preview",
                    p { class: "bulk-paste__preview-title", "Preview" }
                    div {
                        class: "bulk-paste__preview-list",
                        if p.is_empty() {
                            p { class: "field-hint", "Nothing to preview." }
                        } else {
                            for (req, _) in p.iter() {
                                div {
                                    class: "bulk-paste__preview-row",
                                    span { "{fmt_opt_date(req.date)}" }
                                    span { "{req.description}" }
                                    span { "{req.amount} {req.currency}" }
                                    span { "{req.source}" }
                                }
                            }
                        }
                    }
                }
            }

            if let Some(err) = error() {
                p { class: "form-error", style: "margin-top: 12px;", "{err}" }
            }

            div {
                class: "bulk-paste__actions",
                button {
                    class: "btn-ghost",
                    disabled: loading,
                    onclick: preview_clicked,
                    "Preview"
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

fn fmt_opt_date(date: Option<chrono::NaiveDate>) -> String {
    date.map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "Pending".to_string())
}

/// Maps canonical column names to their one-based index.
#[derive(Default, Clone)]
struct ColumnMap {
    date: usize,
    description: usize,
    amount: usize,
    currency: Option<usize>,
    source: Option<usize>,
}

fn default_column_map() -> ColumnMap {
    ColumnMap {
        date: 0,
        description: 1,
        amount: 2,
        currency: Some(3),
        source: Some(4),
    }
}

fn build_column_map(header: &[String]) -> Result<ColumnMap, String> {
    let find = |name: &str| header.iter().position(|h| h.eq_ignore_ascii_case(name));

    let date = find("date").ok_or_else(|| "Header must contain a `date` column".to_string())?;
    let description = find("description")
        .ok_or_else(|| "Header must contain a `description` column".to_string())?;
    let amount =
        find("amount").ok_or_else(|| "Header must contain an `amount` column".to_string())?;

    Ok(ColumnMap {
        date,
        description,
        amount,
        currency: find("currency"),
        source: find("source"),
    })
}

fn split_line(line: &str, sep: char) -> Vec<String> {
    line.split(sep).map(|s| s.trim().to_string()).collect()
}

fn parse_line(
    line: &str,
    sep: char,
    map: &ColumnMap,
    date_format: &str,
) -> Result<CreateTransactionRequest, String> {
    let cols = split_line(line, sep);

    let description =
        get_col(&cols, map.description).ok_or_else(|| "Missing description".to_string())?;
    if description.is_empty() {
        return Err("Description cannot be empty".to_string());
    }

    let amount_str = get_col(&cols, map.amount).ok_or_else(|| "Missing amount".to_string())?;
    let amount = amount_str
        .replace(',', ".")
        .parse::<Decimal>()
        .map_err(|_| format!("Invalid amount: {}", amount_str))?;
    if amount.is_zero() {
        return Err("Amount cannot be zero".to_string());
    }

    let date = if let Some(raw) = get_col(&cols, map.date) {
        if raw.is_empty() {
            None
        } else {
            Some(
                chrono::NaiveDate::parse_from_str(&raw, date_format)
                    .map_err(|_| format!("Invalid date: {}", raw))?,
            )
        }
    } else {
        None
    };

    let currency = map
        .currency
        .and_then(|i| get_col(&cols, i))
        .unwrap_or_else(|| "SEK".to_string())
        .trim()
        .to_ascii_uppercase();
    if currency.is_empty() {
        return Err("Currency cannot be empty".to_string());
    }

    let source = map
        .source
        .and_then(|i| get_col(&cols, i))
        .unwrap_or_else(|| "manual".to_string())
        .trim()
        .to_string();
    if source.is_empty() {
        return Err("Source cannot be empty".to_string());
    }

    Ok(CreateTransactionRequest {
        date,
        description,
        amount,
        currency,
        source,
    })
}

fn get_col(cols: &[String], idx: usize) -> Option<String> {
    cols.get(idx).cloned().filter(|s| !s.is_empty())
}
