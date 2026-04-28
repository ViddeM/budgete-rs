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
            style: "padding: 32px; font-family: sans-serif;",
            h1 { style: "margin: 0 0 24px; font-size: 1.5rem; color: #111827;", "Analytics" }

            // --- Filters ---
            div {
                style: "display: flex; flex-wrap: wrap; gap: 16px; align-items: flex-end; margin-bottom: 28px;",

                div {
                    label { style: "display: block; font-size: 0.8rem; color: #6b7280; margin-bottom: 4px;", "From" }
                    input {
                        r#type: "date",
                        value: date_from(),
                        oninput: move |e| date_from.set(e.value()),
                        style: "padding: 7px 10px; border: 1px solid #d1d5db; border-radius: 8px; font-size: 0.9rem;",
                    }
                }
                div {
                    label { style: "display: block; font-size: 0.8rem; color: #6b7280; margin-bottom: 4px;", "To" }
                    input {
                        r#type: "date",
                        value: date_to(),
                        oninput: move |e| date_to.set(e.value()),
                        style: "padding: 7px 10px; border: 1px solid #d1d5db; border-radius: 8px; font-size: 0.9rem;",
                    }
                }
                if !groups.is_empty() {
                    div {
                        label { style: "display: block; font-size: 0.8rem; color: #6b7280; margin-bottom: 4px;", "Project" }
                        select {
                            style: "padding: 7px 10px; border: 1px solid #d1d5db; border-radius: 8px; font-size: 0.9rem;",
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
            h2 { style: "font-size: 1rem; color: #374151; margin-bottom: 12px;", "Monthly overview" }
            match over_time_res() {
                None => rsx! { p { "Loading…" } },
                Some(Err(e)) => rsx! { p { style: "color: red;", "Error: {e}" } },
                Some(Ok(rows)) if rows.is_empty() => rsx! {
                    p { style: "color: #6b7280;", "No data for selected range." }
                },
                Some(Ok(rows)) => rsx! {
                    div {
                        style: "display: flex; flex-direction: column; gap: 6px; max-width: 600px; margin-bottom: 28px;",
                        div {
                            style: "display: grid; grid-template-columns: 1fr 1fr 1fr; font-size: 0.75rem; font-weight: 700; color: #9ca3af; text-transform: uppercase; padding: 4px 0; border-bottom: 2px solid #e5e7eb;",
                            span { "Period" }
                            span { style: "text-align: right;", "Expenses" }
                            span { style: "text-align: right;", "Income" }
                        }
                        for row in rows.iter() {
                            div {
                                key: "{row.period_label}",
                                style: "display: grid; grid-template-columns: 1fr 1fr 1fr; font-size: 0.9rem; padding: 6px 0; border-bottom: 1px solid #f3f4f6;",
                                span { style: "color: #111827;", "{row.period_label}" }
                                span { style: "text-align: right; color: #dc2626; font-weight: 600;", "{fmt_amount(row.expenses)}" }
                                span { style: "text-align: right; color: #16a34a; font-weight: 600;", "{fmt_amount(row.income)}" }
                            }
                        }
                    }
                },
            }

            // --- By category ---
            h2 { style: "font-size: 1rem; color: #374151; margin-bottom: 12px;", "By category" }
            match category_spend_res() {
                None => rsx! { p { "Loading…" } },
                Some(Err(e)) => rsx! { p { style: "color: red;", "Error: {e}" } },
                Some(Ok(cats)) if cats.is_empty() => rsx! {
                    p { style: "color: #6b7280;", "No categorised transactions in this range." }
                },
                Some(Ok(cats)) => {
                    let cat_groups = build_groups(&cats);
                    let total: f64 = cat_groups.iter().filter_map(|g| g.total.to_f64()).sum();
                    rsx! {
                        div {
                            style: "display: flex; flex-direction: column; gap: 10px; max-width: 480px; margin-bottom: 28px;",
                            for group in cat_groups.iter() {
                                {
                                    let gid = group.id;
                                    let has_subs = !group.subcategories.is_empty();
                                    let pct = group.total.to_f64()
                                        .map(|v| if total > 0.0 { v / total * 100.0 } else { 0.0 })
                                        .unwrap_or(0.0);
                                    let label_style = if has_subs {
                                        "display: flex; justify-content: space-between; cursor: pointer;"
                                    } else {
                                        "display: flex; justify-content: space-between;"
                                    };
                                    let arrow_transform = if expanded_cats().contains(&gid) { "rotate(90deg)" } else { "rotate(0deg)" };
                                    rsx! {
                                        div {
                                            key: "{gid}",

                                            // Top-level row
                                            div {
                                                style: "display: flex; flex-direction: column; gap: 4px;",
                                                div {
                                                    style: "{label_style}",
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
                                                        style: "display: flex; align-items: center; gap: 6px; font-size: 0.9rem; color: #111827;",
                                                        span {
                                                            style: "width: 10px; height: 10px; border-radius: 50%; background: {group.color};",
                                                        }
                                                        "{group.name}"
                                                                        if has_subs {
                                                                            span {
                                                                                style: "display: inline-block; font-size: 0.55rem; color: #9ca3af; margin-left: 2px; transition: transform 0.15s ease; transform: {arrow_transform};",
                                                                                "▶"
                                                                            }
                                                                        }
                                                    }
                                                    span {
                                                        style: "font-weight: 600; font-size: 0.9rem; color: #374151;",
                                                        "{fmt_amount(group.total)}"
                                                    }
                                                }
                                                                // Parent proportional bar — tinted track + solid fill
                                                                div {
                                                                    style: "height: 8px; border-radius: 4px; position: relative; overflow: hidden;",
                                                                    // Tinted track
                                                                    div {
                                                                        style: "position: absolute; inset: 0; background: {group.color}; opacity: 0.15;",
                                                                    }
                                                                    // Solid fill
                                                                    div {
                                                                        style: "position: absolute; top: 0; left: 0; bottom: 0; width: {pct:.1}%; background: {group.color}; border-radius: 4px;",
                                                                    }
                                                                }
                                            }

                                            // Subcategory rows (when expanded)
                                            if expanded_cats().contains(&gid) && has_subs {
                                                div {
                                                    style: "margin-left: 16px; display: flex; flex-direction: column; gap: 6px; margin-top: 6px;",
                                                    for sub in group.subcategories.iter() {
                                                        {
                                                            let sub_pct = sub.total.to_f64()
                                                                .map(|v| if total > 0.0 { v / total * 100.0 } else { 0.0 })
                                                                .unwrap_or(0.0);
                                                            rsx! {
                                                                div {
                                                                    key: "{sub.category_id}",
                                                                    style: "display: flex; flex-direction: column; gap: 3px;",
                                                                    div {
                                                                        style: "display: flex; justify-content: space-between;",
                                                                        span {
                                                                            style: "display: flex; align-items: center; gap: 5px; font-size: 0.82rem; color: #6b7280;",
                                                                            span {
                                                                                style: "width: 8px; height: 8px; border-radius: 50%; background: {sub.category_color};",
                                                                            }
                                                                            "{sub.category_name}"
                                                                        }
                                                                        span {
                                                                            style: "font-size: 0.82rem; font-weight: 500; color: #9ca3af;",
                                                                            "{fmt_amount(sub.total)}"
                                                                        }
                                                                    }
                                                                                    // Subcategory proportional bar
                                                                                    div {
                                                                                        style: "height: 5px; border-radius: 3px; position: relative; overflow: hidden;",
                                                                                        div {
                                                                                            style: "position: absolute; inset: 0; background: {sub.category_color}; opacity: 0.15;",
                                                                                        }
                                                                                        div {
                                                                                            style: "position: absolute; top: 0; left: 0; bottom: 0; width: {sub_pct:.1}%; background: {sub.category_color}; border-radius: 3px;",
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
                class: "btn-ghost",
                style: "padding: 8px 16px; background: #f3f4f6; border: 1px solid #e5e7eb; border-radius: 8px; cursor: pointer; font-size: 0.85rem; margin-bottom: 16px;",
                if show_transactions() { "Hide transactions" } else { "Show all transactions" }
            }

            if show_transactions() {
                match transactions_res() {
                    None => rsx! { p { "Loading…" } },
                    Some(Err(e)) => rsx! { p { style: "color: red;", "Error: {e}" } },
                    Some(Ok(txs)) => rsx! {
                        TransactionList { transactions: txs, classify_action: None }
                    },
                }
            }
        }
    }
}
