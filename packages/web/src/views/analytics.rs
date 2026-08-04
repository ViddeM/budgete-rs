use api::{
    get_spending_by_category, get_spending_over_time, get_transactions, list_groups,
    models::{Group, TransactionFilter},
};
use chrono::{Datelike, Local, NaiveDate};
use dioxus::prelude::*;
use rust_decimal::{prelude::ToPrimitive, Decimal};
use std::collections::HashSet;
use ui::{fmt_amount, tx_amount_color, TransactionList};
use uuid::Uuid;

use super::helpers::build_groups;

// ---------------------------------------------------------------------------
// Helper: CSS class for a net value (positive = green, negative = red)
// ---------------------------------------------------------------------------

fn net_class(value: Decimal) -> &'static str {
    if value >= Decimal::ZERO {
        "time-table__net time-table__net--pos"
    } else {
        "time-table__net time-table__net--neg"
    }
}

#[derive(Clone, PartialEq)]
enum FilterMode {
    Month,
    Custom,
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

#[component]
pub fn Analytics() -> Element {
    let today = Local::now().date_naive();
    let default_from = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();

    // Filter mode — Month or Custom range
    let mut filter_mode = use_signal(|| FilterMode::Month);

    // Month mode state
    let mut sel_month = use_signal(|| today.month());
    let mut sel_year = use_signal(|| today.year());

    // Custom range state
    let mut custom_from = use_signal(|| default_from.to_string());
    let mut custom_to = use_signal(|| today.to_string());

    let mut selected_group: Signal<Option<Uuid>> = use_signal(|| None);
    let mut show_transactions = use_signal(|| false);
    let mut expanded_cats: Signal<HashSet<Uuid>> = use_signal(HashSet::new);

    let groups_res = use_resource(list_groups);
    let groups: Vec<Group> = groups_res().and_then(|r| r.ok()).unwrap_or_default();

    // Derive resolved NaiveDate range from whichever mode is active
    let date_from = use_memo(move || match filter_mode() {
        FilterMode::Month => NaiveDate::from_ymd_opt(sel_year(), sel_month(), 1).unwrap(),
        FilterMode::Custom => {
            NaiveDate::parse_from_str(&custom_from(), "%Y-%m-%d").unwrap_or(default_from)
        }
    });
    let date_to = use_memo(move || match filter_mode() {
        FilterMode::Month => {
            let (y, m) = if sel_month() == 12 {
                (sel_year() + 1, 1)
            } else {
                (sel_year(), sel_month() + 1)
            };
            NaiveDate::from_ymd_opt(y, m, 1)
                .unwrap()
                .pred_opt()
                .unwrap()
        }
        FilterMode::Custom => NaiveDate::parse_from_str(&custom_to(), "%Y-%m-%d").unwrap_or(today),
    });

    let category_spend_res = use_resource(move || {
        let from = date_from();
        let to = date_to();
        let gid = selected_group();
        async move { get_spending_by_category(from, to, gid).await }
    });

    let over_time_res = use_resource(move || {
        let from = date_from();
        let to = date_to();
        let gid = selected_group();
        async move { get_spending_over_time(from, to, gid).await }
    });

    let transactions_res = use_resource(move || {
        let from = date_from();
        let to = date_to();
        let gid = selected_group();
        async move {
            get_transactions(TransactionFilter {
                date_from: Some(from),
                date_to: Some(to),
                group_id: gid,
                exclude_ignored: true,
                ..Default::default()
            })
            .await
        }
    });

    // Totals summed across all returned rows (used by summary cards)
    let (total_expenses, total_income) = match over_time_res() {
        Some(Ok(ref rows)) => {
            let exp: Decimal = rows.iter().map(|r| r.expenses).sum();
            let inc: Decimal = rows.iter().map(|r| r.income).sum();
            (exp, inc)
        }
        _ => (Decimal::ZERO, Decimal::ZERO),
    };
    let net = total_income - total_expenses;

    // Human-readable label for the selected range
    let range_label: String = match filter_mode() {
        FilterMode::Month => format!("{} {}", date_from().format("%B"), sel_year()),
        FilterMode::Custom => format!("{} — {}", date_from(), date_to()),
    };

    rsx! {
        div {
            class: "view",
            h1 { class: "view__title", "Analytics" }

            // --- Filter mode toggle + fields ---
            div { class: "analytics-filter-block",

                // Mode toggle pills
                div { class: "analytics-mode-toggle",
                    button {
                        class: if matches!(filter_mode(), FilterMode::Month) {
                            "mode-pill mode-pill--active"
                        } else {
                            "mode-pill"
                        },
                        onclick: move |_| filter_mode.set(FilterMode::Month),
                        "Month"
                    }
                    button {
                        class: if matches!(filter_mode(), FilterMode::Custom) {
                            "mode-pill mode-pill--active"
                        } else {
                            "mode-pill"
                        },
                        onclick: move |_| filter_mode.set(FilterMode::Custom),
                        "Custom range"
                    }
                }

                div { class: "analytics-filters",

                    // Month mode fields
                    if matches!(filter_mode(), FilterMode::Month) {
                        div { class: "form-field",
                            label { class: "filter-label", "Month" }
                            select {
                                class: "input-std",
                                value: "{sel_month()}",
                                onchange: move |e: Event<FormData>| {
                                    if let Ok(v) = e.value().parse::<u32>() {
                                        sel_month.set(v);
                                    }
                                },
                                for m in 1u32..=12 {
                                    option {
                                        value: "{m}",
                                        selected: sel_month() == m,
                                        // Format any date with this month number to get the name
                                        {
                                            NaiveDate::from_ymd_opt(2000, m, 1)
                                                .map(|d| d.format("%B").to_string())
                                                .unwrap_or_default()
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "form-field",
                            label { class: "filter-label", "Year" }
                            select {
                                class: "input-std",
                                value: "{sel_year()}",
                                onchange: move |e: Event<FormData>| {
                                    if let Ok(v) = e.value().parse::<i32>() {
                                        sel_year.set(v);
                                    }
                                },
                                for y in (today.year() - 5)..=(today.year()) {
                                    option {
                                        value: "{y}",
                                        selected: sel_year() == y,
                                        "{y}"
                                    }
                                }
                            }
                        }
                    }

                    // Custom range fields
                    if matches!(filter_mode(), FilterMode::Custom) {
                        div { class: "form-field",
                            label { class: "filter-label", "From" }
                            input {
                                r#type: "date",
                                class: "input-std",
                                value: custom_from(),
                                oninput: move |e| custom_from.set(e.value()),
                            }
                        }
                        div { class: "form-field",
                            label { class: "filter-label", "To" }
                            input {
                                r#type: "date",
                                class: "input-std",
                                value: custom_to(),
                                oninput: move |e| custom_to.set(e.value()),
                            }
                        }
                    }

                    // Project filter — always visible
                    if !groups.is_empty() {
                        div { class: "form-field",
                            label { class: "filter-label", "Project" }
                            select {
                                class: "input-std",
                                onchange: move |e: Event<FormData>| {
                                    selected_group.set(Uuid::parse_str(&e.value()).ok());
                                },
                                option { value: "", "All projects" }
                                for g in groups.iter() {
                                    option { value: "{g.id}", "{g.name}" }
                                }
                            }
                        }
                    }
                }
            }

            // --- Summary cards ---
            div { class: "analytics-summary",
                div { class: "summary-card summary-card--expense",
                    span { class: "summary-card__label", "Expenses" }
                    span { class: "summary-card__value", "{fmt_amount(total_expenses)}" }
                }
                div { class: "summary-card summary-card--income",
                    span { class: "summary-card__label", "Income" }
                    span { class: "summary-card__value", "{fmt_amount(total_income)}" }
                }
                div {
                    class: "summary-card summary-card--net",
                    style: "color: {tx_amount_color(net)};",
                    span { class: "summary-card__label", "Net" }
                    span { class: "summary-card__value", "{fmt_amount(net)}" }
                }
            }

            // --- Overview section ---
            // Single month: totals already shown in summary cards; just show a label.
            // Multi-month: show month-by-month breakdown table.
            match over_time_res() {
                None => rsx! { p { "Loading…" } },
                Some(Err(e)) => rsx! { p { class: "text-error", "Error: {e}" } },
                Some(Ok(rows)) if rows.is_empty() => rsx! {
                    p { style: "color: var(--text-muted); margin-bottom: 24px;",
                        "No data for selected range."
                    }
                },
                Some(Ok(rows)) if rows.len() == 1 => rsx! {
                    p { class: "analytics-range-label", "Showing: {range_label}" }
                },
                Some(Ok(rows)) => rsx! {
                    h2 { class: "view__section-title", "Monthly breakdown" }
                    div {
                        class: "time-table",
                        div {
                            class: "time-table__header",
                            span { "Period" }
                            span { class: "time-table__header-r", "Expenses" }
                            span { class: "time-table__header-r", "Income" }
                            span { class: "time-table__header-r", "Net" }
                        }
                        for row in rows.iter() {
                            {
                                let row_net = row.income - row.expenses;
                                rsx! {
                                    div {
                                        key: "{row.period_label}",
                                        class: "time-table__row",
                                        span { class: "time-table__period", "{row.period_label}" }
                                        span { class: "time-table__expense", "{fmt_amount(row.expenses)}" }
                                        span { class: "time-table__income",  "{fmt_amount(row.income)}" }
                                        span { class: "{net_class(row_net)}", "{fmt_amount(row_net)}" }
                                    }
                                }
                            }
                        }
                        div {
                            class: "time-table__row time-table__row--total",
                            span { class: "time-table__period", "Total" }
                            span { class: "time-table__expense", "{fmt_amount(total_expenses)}" }
                            span { class: "time-table__income", "{fmt_amount(total_income)}" }
                            span { class: "{net_class(net)}", "{fmt_amount(net)}" }
                        }
                    }
                },
            }

            // --- By category ---
            h2 { class: "view__section-title", "By category" }
            match category_spend_res() {
                None => rsx! { p { "Loading…" } },
                Some(Err(e)) => rsx! { p { class: "text-error", "Error: {e}" } },
                Some(Ok(cats)) if cats.is_empty() => rsx! {
                    p { style: "color: var(--text-muted);", "No categorised transactions in this range." }
                },
                Some(Ok(cats)) => {
                    let cat_groups = build_groups(&cats);
                    let total: f64 = cat_groups.iter().filter_map(|g| g.total.to_f64()).sum();
                    rsx! {
                        div {
                            class: "cat-bars",
                            for group in cat_groups.iter() {
                                {
                                    let gid = group.id;
                                    let has_subs = !group.subcategories.is_empty();
                                    let pct = group.total.to_f64()
                                        .map(|v| if total > 0.0 { v / total * 100.0 } else { 0.0 })
                                        .unwrap_or(0.0);
                                    let label_class = if has_subs {
                                        "cat-bar__label-row cat-bar__label-row--clickable"
                                    } else {
                                        "cat-bar__label-row"
                                    };
                                    let arrow_class = if expanded_cats().contains(&gid) {
                                        "expand-arrow expand-arrow--open"
                                    } else {
                                        "expand-arrow"
                                    };
                                    rsx! {
                                        div {
                                            key: "{gid}",
                                            class: "cat-bar",
                                            div {
                                                class: "cat-bar__label-row-wrap",
                                                style: "display: flex; flex-direction: column; gap: 4px;",
                                                div {
                                                    class: "{label_class}",
                                                    onclick: move |_| {
                                                        if has_subs {
                                                            let mut exp = expanded_cats.write();
                                                            if exp.contains(&gid) {
                                                                exp.remove(&gid);
                                                            } else {
                                                                exp.insert(gid);
                                                            }
                                                        }
                                                    },
                                                    span {
                                                        class: "cat-bar__name-group",
                                                        span {
                                                            class: "color-dot color-dot--md",
                                                            style: "background: {group.color};",
                                                        }
                                                        "{group.name}"
                                                        if has_subs {
                                                            span { class: "{arrow_class}", "▶" }
                                                        }
                                                    }
                                                    span { class: "cat-bar__pct", "{pct:.1}%" }
                                                    span { class: "cat-bar__total", "{fmt_amount(group.total)}" }
                                                }
                                                div {
                                                    class: "cat-bar__track",
                                                    div { class: "cat-bar__tint", style: "background: {group.color};" }
                                                    div {
                                                        class: "cat-bar__fill",
                                                        style: "background: {group.color}; width: {pct:.1}%;",
                                                    }
                                                }
                                            }
                                            if expanded_cats().contains(&gid) && has_subs {
                                                div {
                                                    class: "cat-bar__subs",
                                                    for sub in group.subcategories.iter() {
                                                        {
                                                            let sub_pct = sub.total.to_f64()
                                                                .map(|v| if total > 0.0 { v / total * 100.0 } else { 0.0 })
                                                                .unwrap_or(0.0);
                                                            rsx! {
                                                                div {
                                                                    key: "{sub.category_id}",
                                                                    class: "cat-bar__sub",
                                                                    div {
                                                                        class: "cat-bar__sub-label-row",
                                                                        span {
                                                                            class: "cat-bar__sub-name-group",
                                                                            span {
                                                                                class: "color-dot color-dot--sm",
                                                                                style: "background: {sub.category_color};",
                                                                            }
                                                                            "{sub.category_name}"
                                                                        }
                                                                        span { class: "cat-bar__sub-pct", "{sub_pct:.1}%" }
                                                                        span { class: "cat-bar__sub-total", "{fmt_amount(sub.total)}" }
                                                                    }
                                                                    div {
                                                                        class: "cat-bar__track cat-bar__track--sm",
                                                                        div { class: "cat-bar__tint", style: "background: {sub.category_color};" }
                                                                        div {
                                                                            class: "cat-bar__fill cat-bar__fill--sm",
                                                                            style: "background: {sub.category_color}; width: {sub_pct:.1}%;",
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
            }

            // --- Transaction drill-down toggle ---
            button {
                onclick: move |_| show_transactions.set(!show_transactions()),
                class: "btn-toggle",
                if show_transactions() { "Hide transactions" } else { "Show all transactions" }
            }

            if show_transactions() {
                match transactions_res() {
                    None => rsx! { p { "Loading…" } },
                    Some(Err(e)) => rsx! { p { class: "text-error", "Error: {e}" } },
                    Some(Ok(txs)) => rsx! {
                        TransactionList { transactions: txs, classify_action: None }
                    },
                }
            }
        }
    }
}
