use api::{get_dashboard_stats, models::DashboardStats};
use dioxus::prelude::*;
use rust_decimal::prelude::ToPrimitive;
use ui::StatCard;

#[component]
pub fn Dashboard() -> Element {
    let stats = use_server_future(get_dashboard_stats)?;

    match stats() {
        None => rsx! { p { "Loading…" } },
        Some(Err(e)) => rsx! { p { style: "color: red;", "Error: {e}" } },
        Some(Ok(s)) => rsx! { DashboardContent { stats: s } },
    }
}

#[component]
fn DashboardContent(stats: DashboardStats) -> Element {
    let mom = stats.mom_delta_pct.and_then(|d| d.to_f64()).map(|p| {
        let sign = if p >= 0.0 { "+" } else { "" };
        format!("{sign}{p:.1}% vs last month")
    });

    rsx! {
        div {
            style: "padding: 32px; font-family: sans-serif;",
            h1 { style: "margin: 0 0 24px; font-size: 1.5rem; color: #111827;", "Dashboard" }

            div {
                style: "display: flex; flex-wrap: wrap; gap: 16px; margin-bottom: 32px;",

                StatCard {
                    label: "Expenses this month".to_string(),
                    value: format!("{:.0} SEK", stats.month_expenses),
                    sub_label: mom,
                }
                StatCard {
                    label: "Income this month".to_string(),
                    value: format!("{:.0} SEK", stats.month_income),
                    sub_label: None,
                }
                StatCard {
                    label: "Unprocessed".to_string(),
                    value: stats.unprocessed_count.to_string(),
                    sub_label: Some("transactions need a category".to_string()),
                }
            }

            if !stats.top_categories.is_empty() {
                h2 { style: "font-size: 1rem; color: #374151; margin-bottom: 12px;", "Top categories this month" }
                div {
                    style: "display: flex; flex-direction: column; gap: 8px; max-width: 480px;",
                    for cat in stats.top_categories.iter() {
                        div {
                            key: "{cat.category_id}",
                            style: "display: flex; justify-content: space-between; align-items: center; padding: 8px 12px; background: #f9fafb; border-radius: 8px;",
                            span {
                                style: "display: flex; align-items: center; gap: 8px;",
                                span {
                                    style: "width: 12px; height: 12px; border-radius: 50%; background: {cat.category_color};",
                                }
                                span { style: "font-size: 0.9rem; color: #111827;", "{cat.category_name}" }
                            }
                            span { style: "font-weight: 600; color: #374151;", "{cat.total:.0} SEK" }
                        }
                    }
                }
            }
        }
    }
}
