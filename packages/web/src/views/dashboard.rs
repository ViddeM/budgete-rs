use api::{
    get_dashboard_stats,
    models::{CategorySpend, DashboardStats},
};
use dioxus::prelude::*;
use rust_decimal::{prelude::ToPrimitive, Decimal};
use std::collections::{HashMap, HashSet};
use ui::{fmt_amount, StatCard};
use uuid::Uuid;

#[component]
pub fn Dashboard() -> Element {
    let stats = use_server_future(get_dashboard_stats)?;

    match stats() {
        None => rsx! { p { "Loading…" } },
        Some(Err(e)) => rsx! { p { style: "color: red;", "Error: {e}" } },
        Some(Ok(s)) => rsx! { DashboardContent { stats: s } },
    }
}

// ---------------------------------------------------------------------------
// Grouping helper
// ---------------------------------------------------------------------------

struct CategoryGroup {
    id: Uuid,
    name: String,
    color: String,
    total: Decimal,
    subcategories: Vec<CategorySpend>,
}

/// Group a flat `CategorySpend` list by top-level category, summing subcategory
/// totals under their parent. Returns groups sorted by total descending.
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
// Components
// ---------------------------------------------------------------------------

#[component]
fn DashboardContent(stats: DashboardStats) -> Element {
    let mom = stats.mom_delta_pct.and_then(|d| d.to_f64()).map(|p| {
        let sign = if p >= 0.0 { "+" } else { "" };
        format!("{sign}{p:.1}% vs last month")
    });

    let balance = stats.month_income - stats.month_expenses;
    let balance_color = if balance >= Decimal::ZERO { "#16a34a" } else { "#dc2626" };

    let top5: Vec<CategoryGroup> = build_groups(&stats.top_categories)
        .into_iter()
        .take(5)
        .collect();

    let mut expanded: Signal<HashSet<Uuid>> = use_signal(HashSet::new);

    rsx! {
        div {
            style: "padding: 32px; font-family: sans-serif;",
            h1 { style: "margin: 0 0 24px; font-size: 1.5rem; color: #111827;", "Dashboard" }

            div {
                style: "display: flex; flex-wrap: wrap; gap: 16px; margin-bottom: 32px;",

                StatCard {
                    label: "Expenses this month".to_string(),
                    value: fmt_amount(stats.month_expenses),
                    sub_label: mom,
                    value_color: "#dc2626".to_string(),
                }
                StatCard {
                    label: "Income this month".to_string(),
                    value: fmt_amount(stats.month_income),
                    sub_label: None,
                    value_color: "#16a34a".to_string(),
                }
                StatCard {
                    label: "Balance this month".to_string(),
                    value: fmt_amount(balance),
                    sub_label: None,
                    value_color: balance_color.to_string(),
                }
                StatCard {
                    label: "Unprocessed".to_string(),
                    value: stats.unprocessed_count.to_string(),
                    sub_label: Some("transactions need a category".to_string()),
                }
            }

            if !top5.is_empty() {
                h2 { style: "font-size: 1rem; color: #374151; margin-bottom: 12px;", "Top categories this month" }
                div {
                    style: "display: flex; flex-direction: column; gap: 6px; max-width: 480px;",
                    for group in top5.iter() {
                        {
                            let gid = group.id;
                            let has_subs = !group.subcategories.is_empty();
                            let row_style = if has_subs {
                                "display: flex; justify-content: space-between; align-items: center; padding: 8px 12px; background: #f9fafb; border-radius: 8px; cursor: pointer;"
                            } else {
                                "display: flex; justify-content: space-between; align-items: center; padding: 8px 12px; background: #f9fafb; border-radius: 8px;"
                            };
                            let arrow_transform = if expanded().contains(&gid) { "rotate(90deg)" } else { "rotate(0deg)" };
                            rsx! {
                                div {
                                    key: "{gid}",

                                    // Top-level row
                                    div {
                                        style: "{row_style}",
                                        onclick: move |_| {
                                            if has_subs {
                                                let mut exp = expanded.write();
                                                if exp.contains(&gid) {
                                                    exp.remove(&gid);
                                                } else {
                                                    exp.insert(gid);
                                                }
                                            }
                                        },
                                        span {
                                            style: "display: flex; align-items: center; gap: 8px;",
                                            span {
                                                style: "width: 12px; height: 12px; border-radius: 50%; background: {group.color};",
                                            }
                                            span {
                                                style: "font-size: 0.9rem; color: #111827;",
                                                "{group.name}"
                                            }
                                                        if has_subs {
                                                            span {
                                                                style: "display: inline-block; font-size: 0.55rem; color: #9ca3af; transition: transform 0.15s ease; transform: {arrow_transform};",
                                                                "▶"
                                                            }
                                                        }
                                        }
                                        span {
                                            style: "font-weight: 600; color: #374151;",
                                            "{fmt_amount(group.total)}"
                                        }
                                    }

                                    // Subcategory rows (when expanded)
                                    if expanded().contains(&gid) && has_subs {
                                        div {
                                            style: "margin-left: 20px; display: flex; flex-direction: column; gap: 3px; margin-top: 3px;",
                                            for sub in group.subcategories.iter() {
                                                div {
                                                    key: "{sub.category_id}",
                                                    style: "display: flex; justify-content: space-between; align-items: center; padding: 6px 10px; background: #f3f4f6; border-radius: 6px;",
                                                    span {
                                                        style: "display: flex; align-items: center; gap: 6px;",
                                                        span {
                                                            style: "width: 8px; height: 8px; border-radius: 50%; background: {sub.category_color};",
                                                        }
                                                        span {
                                                            style: "font-size: 0.85rem; color: #374151;",
                                                            "{sub.category_name}"
                                                        }
                                                    }
                                                    span {
                                                        style: "font-size: 0.85rem; font-weight: 500; color: #6b7280;",
                                                        "{fmt_amount(sub.total)}"
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
