use api::{get_dashboard_stats, models::DashboardStats};
use dioxus::prelude::*;
use rust_decimal::{prelude::ToPrimitive, Decimal};
use std::collections::HashSet;
use ui::{fmt_amount, StatCard};
use uuid::Uuid;

use super::helpers::{build_groups, CategoryGroup};

#[component]
pub fn Dashboard() -> Element {
    let stats = use_server_future(get_dashboard_stats)?;

    match stats() {
        None => rsx! { p { "Loading…" } },
        Some(Err(e)) => rsx! { p { class: "text-error", "Error: {e}" } },
        Some(Ok(s)) => rsx! { DashboardContent { stats: s } },
    }
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
    let balance_color = if balance >= Decimal::ZERO {
        "#16a34a"
    } else {
        "#dc2626"
    };

    let top5: Vec<CategoryGroup> = build_groups(&stats.top_categories)
        .into_iter()
        .take(5)
        .collect();

    let mut expanded: Signal<HashSet<Uuid>> = use_signal(HashSet::new);

    rsx! {
        div {
            class: "view",
            h1 { class: "view__title", "Dashboard" }

            div {
                class: "dash-stats",

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
                h2 { class: "view__section-title", "Top categories this month" }
                div {
                    class: "cat-tree",
                    for group in top5.iter() {
                        {
                            let gid = group.id;
                            let has_subs = !group.subcategories.is_empty();
                            let row_class = if has_subs {
                                "cat-tree__row cat-tree__row--clickable"
                            } else {
                                "cat-tree__row"
                            };
                            let arrow_class = if expanded().contains(&gid) {
                                "expand-arrow expand-arrow--open"
                            } else {
                                "expand-arrow"
                            };
                            rsx! {
                                div {
                                    key: "{gid}",

                                    // Top-level row
                                    div {
                                        class: "{row_class}",
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
                                            class: "cat-tree__name-group",
                                            span {
                                                class: "color-dot color-dot--lg",
                                                style: "background: {group.color};",
                                            }
                                            span { class: "cat-tree__name", "{group.name}" }
                                            if has_subs {
                                                span { class: "{arrow_class}", "▶" }
                                            }
                                        }
                                        span { class: "cat-tree__total", "{fmt_amount(group.total)}" }
                                    }

                                    // Subcategory rows (when expanded)
                                    if expanded().contains(&gid) && has_subs {
                                        div {
                                            class: "cat-tree__subs",
                                            for sub in group.subcategories.iter() {
                                                div {
                                                    key: "{sub.category_id}",
                                                    class: "cat-tree__sub-row",
                                                    span {
                                                        class: "cat-tree__sub-name-group",
                                                        span {
                                                            class: "color-dot color-dot--sm",
                                                            style: "background: {sub.category_color};",
                                                        }
                                                        span { class: "cat-tree__sub-label", "{sub.category_name}" }
                                                    }
                                                    span { class: "cat-tree__sub-total", "{fmt_amount(sub.total)}" }
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
