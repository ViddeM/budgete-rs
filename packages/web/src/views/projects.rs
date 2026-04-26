use api::models::{Group, Transaction, TransactionFilter};
use api::{add_to_group, create_group, delete_group, get_transactions, list_groups, remove_from_group};
use dioxus::prelude::*;
use rust_decimal::Decimal;
use std::collections::HashSet;
use ui::{fmt_amount, fmt_tx_amount};
use uuid::Uuid;

#[component]
pub fn Projects() -> Element {
    let mut projects_res = use_resource(list_groups);
    let mut selected_id: Signal<Option<Uuid>> = use_signal(|| None);
    let mut show_add = use_signal(|| false);
    let mut add_search = use_signal(String::new);

    // New project form
    let mut new_name = use_signal(String::new);
    let mut new_desc = use_signal(String::new);
    let mut create_error: Signal<Option<String>> = use_signal(|| None);

    // Transactions in the selected project — re-runs when selected_id changes.
    let mut project_txs_res = use_resource(move || {
        let gid = selected_id();
        async move {
            match gid {
                Some(id) => {
                    get_transactions(TransactionFilter {
                        group_id: Some(id),
                        ..Default::default()
                    })
                    .await
                }
                None => Ok(vec![]),
            }
        }
    });

    // All non-pending transactions — used by the "add" panel.
    let all_txs_res = use_resource(|| async {
        get_transactions(TransactionFilter::default()).await
    });

    let create_project = move |_| async move {
        create_error.set(None);
        let name = new_name().trim().to_string();
        if name.is_empty() {
            create_error.set(Some("Name is required.".to_string()));
            return;
        }
        match create_group(name, new_desc()).await {
            Ok(_) => {
                new_name.set(String::new());
                new_desc.set(String::new());
                projects_res.restart();
            }
            Err(e) => create_error.set(Some(e.to_string())),
        }
    };

    // ── Derived state ─────────────────────────────────────────────────────────

    let projects: Vec<Group> = projects_res().and_then(|r| r.ok()).unwrap_or_default();

    let selected_project: Option<Group> =
        projects.iter().find(|g| Some(g.id) == selected_id()).cloned();

    let project_txs: Vec<Transaction> =
        project_txs_res().and_then(|r| r.ok()).unwrap_or_default();

    let project_tx_ids: HashSet<Uuid> = project_txs.iter().map(|t| t.id).collect();

    let available_txs: Vec<Transaction> = all_txs_res()
        .and_then(|r| r.ok())
        .unwrap_or_default()
        .into_iter()
        .filter(|t| !t.is_pending && !project_tx_ids.contains(&t.id))
        .collect();

    let search_q = add_search().to_lowercase();
    let filtered_available: Vec<&Transaction> = available_txs
        .iter()
        .filter(|t| {
            if search_q.is_empty() {
                return true;
            }
            t.description.to_lowercase().contains(&search_q)
                || t.date
                    .map(|d| d.to_string())
                    .unwrap_or_default()
                    .contains(&search_q)
        })
        .collect();

    // Stats for the selected project
    let total_expense: Decimal = project_txs
        .iter()
        .filter(|t| t.amount < Decimal::ZERO)
        .map(|t| -t.amount)
        .sum();
    let total_income: Decimal = project_txs
        .iter()
        .filter(|t| t.amount > Decimal::ZERO)
        .map(|t| t.amount)
        .sum();
    let tx_count = project_txs.len();

    rsx! {
        div {
            style: "padding: 32px; font-family: sans-serif;",
            h1 { style: "margin: 0 0 28px; font-size: 1.5rem; color: #111827;", "Projects" }

            div {
                style: "display: flex; gap: 32px; align-items: flex-start;",

                // ── Left column: project list + create ────────────────────────
                div {
                    style: "width: 280px; flex-shrink: 0;",

                    // Create form
                    div {
                        style: "background: #f9fafb; border: 1px solid #e5e7eb; border-radius: 12px; padding: 16px; margin-bottom: 20px;",
                        p {
                            style: "font-size: 0.75rem; font-weight: 700; color: #6b7280; margin: 0 0 10px; text-transform: uppercase; letter-spacing: 0.05em;",
                            "New project"
                        }
                        input {
                            r#type: "text",
                            value: new_name(),
                            oninput: move |e| new_name.set(e.value()),
                            placeholder: "Name",
                            style: "width: 100%; padding: 7px 10px; border: 1px solid #d1d5db; border-radius: 8px; font-size: 0.9rem; box-sizing: border-box; margin-bottom: 8px;",
                        }
                        input {
                            r#type: "text",
                            value: new_desc(),
                            oninput: move |e| new_desc.set(e.value()),
                            placeholder: "Description (optional)",
                            style: "width: 100%; padding: 7px 10px; border: 1px solid #d1d5db; border-radius: 8px; font-size: 0.9rem; box-sizing: border-box; margin-bottom: 8px;",
                        }
                        button {
                            onclick: create_project,
                            style: "width: 100%; padding: 8px; background: #1e293b; color: #f1f5f9; border: none; border-radius: 8px; cursor: pointer; font-size: 0.85rem; font-weight: 600;",
                            "Create"
                        }
                        if let Some(err) = create_error() {
                            p { style: "color: #dc2626; font-size: 0.8rem; margin: 6px 0 0;", "{err}" }
                        }
                    }

                    // Project cards
                    div {
                        style: "display: flex; flex-direction: column; gap: 8px;",
                        match projects_res() {
                            None => rsx! { p { style: "color: #6b7280; font-size: 0.9rem;", "Loading…" } },
                            Some(Err(e)) => rsx! { p { style: "color: #dc2626;", "Error: {e}" } },
                            Some(Ok(_)) if projects.is_empty() => rsx! {
                                p { style: "color: #6b7280; font-size: 0.9rem;", "No projects yet." }
                            },
                            Some(Ok(_)) => rsx! {
                                for project in projects.iter() {
                                    {
                                        let pid = project.id;
                                        let pname = project.name.clone();
                                        let pdesc = project.description.clone();
                                        let is_selected = selected_id() == Some(pid);
                                        let card_bg = if is_selected { "#1e293b" } else { "#fff" };
                                        let card_border = if is_selected { "#1e293b" } else { "#e5e7eb" };
                                        let name_color = if is_selected { "#f1f5f9" } else { "#111827" };
                                        let desc_color = if is_selected { "#94a3b8" } else { "#6b7280" };
                                        rsx! {
                                            div {
                                                key: "{pid}",
                                                style: "background: {card_bg}; border: 1px solid {card_border}; border-radius: 10px; display: flex; align-items: center; overflow: hidden;",
                                                // Clickable name area
                                                div {
                                                    style: "flex: 1; padding: 11px 14px; cursor: pointer; min-width: 0;",
                                                    onclick: move |_| {
                                                        selected_id.set(Some(pid));
                                                        show_add.set(false);
                                                        add_search.set(String::new());
                                                    },
                                                    span {
                                                        style: "display: block; font-size: 0.9rem; font-weight: 600; color: {name_color};",
                                                        "{pname}"
                                                    }
                                                    if !pdesc.is_empty() {
                                                        span {
                                                            style: "display: block; font-size: 0.78rem; color: {desc_color}; margin-top: 2px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                                                            "{pdesc}"
                                                        }
                                                    }
                                                }
                                                // Delete button (sibling — no bubbling issue)
                                                button {
                                                    onclick: move |_| async move {
                                                        let _ = delete_group(pid).await;
                                                        if selected_id() == Some(pid) {
                                                            selected_id.set(None);
                                                        }
                                                        projects_res.restart();
                                                    },
                                                    style: "background: transparent; border: none; border-left: 1px solid {card_border}; color: #dc2626; cursor: pointer; font-size: 1rem; padding: 0 12px; align-self: stretch; flex-shrink: 0;",
                                                    "×"
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                        }
                    }
                }

                // ── Right panel: selected project ─────────────────────────────
                div {
                    style: "flex: 1; min-width: 0;",

                    match selected_project.clone() {
                        None => rsx! {
                            p { style: "color: #9ca3af; font-size: 0.95rem; padding: 24px 0;",
                                "Select a project to manage its transactions."
                            }
                        },
                        Some(project) => rsx! {

                            // Project header + stats
                            div {
                                style: "margin-bottom: 20px;",
                                h2 {
                                    style: "margin: 0 0 4px; font-size: 1.1rem; color: #111827;",
                                    "{project.name}"
                                }
                                if !project.description.is_empty() {
                                    p {
                                        style: "margin: 0 0 10px; font-size: 0.875rem; color: #6b7280;",
                                        "{project.description}"
                                    }
                                }
                                div {
                                    style: "display: flex; gap: 20px; font-size: 0.85rem; flex-wrap: wrap;",
                                    span { style: "color: #374151;", "{tx_count} transactions" }
                                    if total_expense > Decimal::ZERO {
                                        span {
                                            style: "color: #dc2626; font-weight: 600;",
                                            "−{fmt_amount(total_expense)}"
                                        }
                                    }
                                    if total_income > Decimal::ZERO {
                                        span {
                                            style: "color: #16a34a; font-weight: 600;",
                                            "+{fmt_amount(total_income)}"
                                        }
                                    }
                                }
                            }

                            // Transactions already in the project
                            match project_txs_res() {
                                None => rsx! { p { style: "color: #6b7280;", "Loading…" } },
                                Some(Err(e)) => rsx! { p { style: "color: #dc2626;", "Error: {e}" } },
                                Some(Ok(_)) if project_txs.is_empty() => rsx! {
                                    p {
                                        style: "color: #6b7280; margin-bottom: 16px;",
                                        "No transactions yet — add some below."
                                    }
                                },
                                Some(Ok(_)) => rsx! {
                                    div {
                                        style: "display: flex; flex-direction: column; gap: 6px; margin-bottom: 20px;",
                                        for tx in project_txs.iter() {
                                            {
                                                let tid = tx.id;
                                                let project_id = project.id;
                                                let date_str = tx.date
                                                    .map(|d| d.format("%Y-%m-%d").to_string())
                                                    .unwrap_or_else(|| "Pending".to_string());
                                                let amount_str = fmt_tx_amount(tx.amount, &tx.currency);
                                                let amount_color = if tx.amount >= Decimal::ZERO {
                                                    "#16a34a"
                                                } else {
                                                    "#dc2626"
                                                };
                                                let desc = tx.description.clone();
                                                let cat_name =
                                                    tx.category.as_ref().map(|c| c.name.clone());
                                                rsx! {
                                                    div {
                                                        key: "{tid}",
                                                        style: "display: flex; align-items: center; gap: 10px; padding: 10px 14px; background: #fff; border: 1px solid #e5e7eb; border-radius: 10px;",
                                                        span {
                                                            style: "min-width: 90px; font-size: 0.78rem; color: #9ca3af;",
                                                            "{date_str}"
                                                        }
                                                        span {
                                                            style: "flex: 1; font-size: 0.88rem; color: #374151; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                                                            "{desc}"
                                                        }
                                                        if let Some(cat) = cat_name {
                                                            span {
                                                                style: "font-size: 0.75rem; color: #6b7280; white-space: nowrap;",
                                                                "{cat}"
                                                            }
                                                        }
                                                        span {
                                                            style: "font-size: 0.88rem; font-weight: 600; color: {amount_color}; white-space: nowrap;",
                                                            "{amount_str}"
                                                        }
                                                        button {
                                                            onclick: move |_| async move {
                                                                let _ = remove_from_group(tid, project_id).await;
                                                                project_txs_res.restart();
                                                            },
                                                            style: "padding: 3px 10px; background: transparent; color: #6b7280; border: 1px solid #e5e7eb; border-radius: 6px; cursor: pointer; font-size: 0.75rem; white-space: nowrap; flex-shrink: 0;",
                                                            "Remove"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                },
                            }

                            // Add transactions panel
                            button {
                                onclick: move |_| {
                                    show_add.set(!show_add());
                                    add_search.set(String::new());
                                },
                                style: "padding: 7px 16px; background: #f3f4f6; border: 1px solid #e5e7eb; border-radius: 8px; cursor: pointer; font-size: 0.85rem; font-weight: 500; margin-bottom: 12px;",
                                if show_add() { "▲ Hide add panel" } else { "▼ Add transactions" }
                            }

                            if show_add() {
                                // Search input
                                input {
                                    r#type: "text",
                                    value: add_search(),
                                    oninput: move |e| add_search.set(e.value()),
                                    placeholder: "Search by description or date…",
                                    style: "width: 100%; max-width: 400px; padding: 7px 10px; border: 1px solid #d1d5db; border-radius: 8px; font-size: 0.9rem; box-sizing: border-box; margin-bottom: 10px;",
                                }
                                if filtered_available.is_empty() {
                                    p {
                                        style: "color: #6b7280; font-size: 0.9rem;",
                                        if available_txs.is_empty() {
                                            "All transactions are already in this project."
                                        } else {
                                            "No transactions match your search."
                                        }
                                    }
                                } else {
                                    div {
                                        style: "display: flex; flex-direction: column; gap: 4px; max-height: 480px; overflow-y: auto;",
                                        for tx in filtered_available.iter() {
                                            {
                                                let tid = tx.id;
                                                let project_id = project.id;
                                                let date_str = tx.date
                                                    .map(|d| d.format("%Y-%m-%d").to_string())
                                                    .unwrap_or_else(|| "Pending".to_string());
                                                let amount_str = fmt_tx_amount(tx.amount, &tx.currency);
                                                let amount_color = if tx.amount >= Decimal::ZERO {
                                                    "#16a34a"
                                                } else {
                                                    "#dc2626"
                                                };
                                                let desc = tx.description.clone();
                                                let cat_name =
                                                    tx.category.as_ref().map(|c| c.name.clone());
                                                rsx! {
                                                    div {
                                                        key: "{tid}",
                                                        style: "display: flex; align-items: center; gap: 10px; padding: 8px 14px; background: #fafafa; border: 1px solid #f3f4f6; border-radius: 8px;",
                                                        span {
                                                            style: "min-width: 90px; font-size: 0.78rem; color: #9ca3af;",
                                                            "{date_str}"
                                                        }
                                                        span {
                                                            style: "flex: 1; font-size: 0.85rem; color: #374151; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                                                            "{desc}"
                                                        }
                                                        if let Some(cat) = cat_name {
                                                            span {
                                                                style: "font-size: 0.75rem; color: #6b7280; white-space: nowrap;",
                                                                "{cat}"
                                                            }
                                                        }
                                                        span {
                                                            style: "font-size: 0.85rem; font-weight: 600; color: {amount_color}; white-space: nowrap;",
                                                            "{amount_str}"
                                                        }
                                                        button {
                                                            onclick: move |_| async move {
                                                                let _ = add_to_group(tid, project_id).await;
                                                                project_txs_res.restart();
                                                            },
                                                            style: "padding: 3px 10px; background: #1e293b; color: #f1f5f9; border: none; border-radius: 6px; cursor: pointer; font-size: 0.75rem; white-space: nowrap; flex-shrink: 0;",
                                                            "Add"
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
                }
            }
        }
    }
}
