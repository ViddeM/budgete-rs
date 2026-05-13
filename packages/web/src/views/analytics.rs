use api::{
    get_spending_by_category, get_spending_over_time, get_transactions, list_groups,
    models::{CategorySpend, Group, TransactionFilter},
};
use chrono::{Datelike, Local, NaiveDate};
use dioxus::prelude::*;
use rust_decimal::{prelude::ToPrimitive, Decimal};
use std::collections::{HashMap, HashSet};
use ui::{fmt_amount, TransactionList};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Grouping helper (mirrors dashboard.rs)
// ---------------------------------------------------------------------------

struct CategoryGroup {
    id: Uuid,
    name: String,
    color: String,
    total: Decimal,
    subcategories: Vec<CategorySpend>,
}

fn build_groups(cats: &[CategorySpend]) -> Vec<CategoryGroup> {
    let mut map: HashMap<Uuid, CategoryGroup> = HashMap::new();

    for cat in cats {
        let (gid, gname, gcolor) = if let Some(pid) = cat.parent_id {
            (
                pid,
                cat.parent_name.clone().unwrap_or_default(),
                cat.parent_color
                    .clone()
                    .unwrap_or_else(|| cat.category_color.clone()),
            )
        } else {
            (
                cat.category_id,
                cat.category_name.clone(),
                cat.category_color.clone(),
            )
        };

        let entry = map.entry(gid).or_insert_with(|| CategoryGroup {
            id: gid,
            name: gname,
            color: gcolor,
            total: Decimal::ZERO,
            subcategories: vec![],
        });

        entry.total += cat.total;

        if cat.parent_id.is_some() {
            entry.subcategories.push(cat.clone());
        }
    }

    let mut groups: Vec<CategoryGroup> = map.into_values().collect();
    groups.sort_by(|a, b| b.total.cmp(&a.total));
    groups
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

#[component]
pub fn Analytics() -> Element {
    let today = Local::now().date_naive();
    let default_from = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();

    let mut date_from = use_signal(|| default_from.to_string());
    let mut date_to = use_signal(|| today.to_string());
    let mut selected_group: Signal<Option<Uuid>> = use_signal(|| None);
    let mut show_transactions = use_signal(|| false);
    let mut expanded_cats: Signal<HashSet<Uuid>> = use_signal(HashSet::new);

    let groups_res = use_resource(list_groups);
    let groups: Vec<Group> = groups_res().and_then(|r| r.ok()).unwrap_or_default();

    // Parse dates from signals
    let parsed_from = use_memo(move || NaiveDate::parse_from_str(&date_from(), "%Y-%m-%d").ok());
    let parsed_to = use_memo(move || NaiveDate::parse_from_str(&date_to(), "%Y-%m-%d").ok());

    let category_spend_res = use_resource(move || {
        let from = parsed_from().unwrap_or(default_from);
        let to = parsed_to().unwrap_or(today);
        let gid = selected_group();
        async move { get_spending_by_category(from, to, gid).await }
    });

    let over_time_res = use_resource(move || {
        let from = parsed_from().unwrap_or(default_from);
        let to = parsed_to().unwrap_or(today);
        let gid = selected_group();
        async move { get_spending_over_time(from, to, gid).await }
    });

    let transactions_res = use_resource(move || {
        let from = parsed_from();
        let to = parsed_to();
        let gid = selected_group();
        async move {
            get_transactions(TransactionFilter {
                date_from: from,
                date_to: to,
                group_id: gid,
                ..Default::default()
            })
            .await
        }
    });

    rsx! {
        div {
            class: "view",
            h1 { class: "view__title", "Analytics" }

            // --- Filters ---
            div {
                class: "analytics-filters",

                div {
                    label { class: "filter-label", "From" }
                    input {
                        r#type: "date",
                        value: date_from(),
                        oninput: move |e| date_from.set(e.value()),
                        class: "input-std",
                    }
                }
                div {
                    label { class: "filter-label", "To" }
                    input {
                        r#type: "date",
                        value: date_to(),
                        oninput: move |e| date_to.set(e.value()),
                        class: "input-std",
                    }
                }
                if !groups.is_empty() {
                    div {
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

            // --- Spending over time ---
            h2 { class: "view__section-title", "Monthly overview" }
            match over_time_res() {
                None => rsx! { p { "Loading…" } },
                Some(Err(e)) => rsx! { p { class: "text-error", "Error: {e}" } },
                Some(Ok(rows)) if rows.is_empty() => rsx! {
                    p { style: "color: var(--text-muted);", "No data for selected range." }
                },
                Some(Ok(rows)) => rsx! {
                    div {
                        class: "time-table",
                        div {
                            class: "time-table__header",
                            span { "Period" }
                            span { class: "time-table__header-r", "Expenses" }
                            span { class: "time-table__header-r", "Income" }
                        }
                        for row in rows.iter() {
                            div {
                                key: "{row.period_label}",
                                class: "time-table__row",
                                span { class: "time-table__period", "{row.period_label}" }
                                span { class: "time-table__expense", "{fmt_amount(row.expenses)}" }
                                span { class: "time-table__income",  "{fmt_amount(row.income)}" }
                            }
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

                                            // Top-level row
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
                                                    span { class: "cat-bar__total", "{fmt_amount(group.total)}" }
                                                }
                                                // Parent proportional bar
                                                div {
                                                    class: "cat-bar__track",
                                                    div { class: "cat-bar__tint", style: "background: {group.color};" }
                                                    div {
                                                        class: "cat-bar__fill",
                                                        style: "background: {group.color}; width: {pct:.1}%;",
                                                    }
                                                }
                                            }

                                            // Subcategory rows (when expanded)
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
                                                                        span { class: "cat-bar__sub-total", "{fmt_amount(sub.total)}" }
                                                                    }
                                                                    // Subcategory proportional bar
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
