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
            class: "view",
            h1 { class: "view__title", style: "margin-bottom: 28px;", "Projects" }

            div {
                class: "two-col",

                // ── Left column: project list + create ────────────────────────
                div {
                    class: "projects-col-left",

                    // Create form
                    div {
                        class: "form-card",
                        p { class: "form-card__title", style: "margin-bottom: 10px;", "New project" }
                        input {
                            r#type: "text",
                            value: new_name(),
                            oninput: move |e| new_name.set(e.value()),
                            placeholder: "Name",
                            class: "input-std input-std--full input-std--mb",
                        }
                        input {
                            r#type: "text",
                            value: new_desc(),
                            oninput: move |e| new_desc.set(e.value()),
                            placeholder: "Description (optional)",
                            class: "input-std input-std--full input-std--mb",
                        }
                        button {
                            onclick: create_project,
                            class: "btn-primary btn-primary--full",
                            "Create"
                        }
                        if let Some(err) = create_error() {
                            p { class: "form-error", "{err}" }
                        }
                    }

                    // Project cards
                    div {
                        class: "project-cards",
                        match projects_res() {
                            None => rsx! { p { style: "color: var(--text-muted); font-size: 0.9rem;", "Loading…" } },
                            Some(Err(e)) => rsx! { p { class: "text-error", "Error: {e}" } },
                            Some(Ok(_)) if projects.is_empty() => rsx! {
                                p { style: "color: var(--text-muted); font-size: 0.9rem;", "No projects yet." }
                            },
                            Some(Ok(_)) => rsx! {
                                for project in projects.iter() {
                                    {
                                        let pid = project.id;
                                        let pname = project.name.clone();
                                        let pdesc = project.description.clone();
                                        let is_selected = selected_id() == Some(pid);
                                        let card_class = if is_selected {
                                            "project-card project-card--selected"
                                        } else {
                                            "project-card"
                                        };
                                        rsx! {
                                            div {
                                                key: "{pid}",
                                                class: "{card_class}",
                                                div {
                                                    class: "project-card__body",
                                                    onclick: move |_| {
                                                        selected_id.set(Some(pid));
                                                        show_add.set(false);
                                                        add_search.set(String::new());
                                                    },
                                                    span { class: "project-card__name", "{pname}" }
                                                    if !pdesc.is_empty() {
                                                        span { class: "project-card__desc", "{pdesc}" }
                                                    }
                                                }
                                                button {
                                                    onclick: move |_| async move {
                                                        let _ = delete_group(pid).await;
                                                        if selected_id() == Some(pid) {
                                                            selected_id.set(None);
                                                        }
                                                        projects_res.restart();
                                                    },
                                                    class: "project-card__delete",
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
                    class: "two-col__right",

                    match selected_project.clone() {
                        None => rsx! {
                            p { style: "color: var(--text-dim); font-size: 0.95rem; padding: 24px 0;",
                                "Select a project to manage its transactions."
                            }
                        },
                        Some(project) => rsx! {

                            // Project header + stats
                            div {
                                style: "margin-bottom: 20px;",
                                h2 {
                                    style: "margin: 0 0 4px; font-size: 1.1rem; color: var(--text-primary);",
                                    "{project.name}"
                                }
                                if !project.description.is_empty() {
                                    p {
                                        style: "margin: 0 0 10px; font-size: 0.875rem; color: var(--text-muted);",
                                        "{project.description}"
                                    }
                                }
                                div {
                                    class: "project-stats",
                                    span { style: "color: var(--text-secondary);", "{tx_count} transactions" }
                                    if total_expense > Decimal::ZERO {
                                        span { style: "color: #dc2626; font-weight: 600;", "−{fmt_amount(total_expense)}" }
                                    }
                                    if total_income > Decimal::ZERO {
                                        span { style: "color: #16a34a; font-weight: 600;", "+{fmt_amount(total_income)}" }
                                    }
                                }
                            }

                            // Transactions already in the project
                            match project_txs_res() {
                                None => rsx! { p { style: "color: var(--text-muted);", "Loading…" } },
                                Some(Err(e)) => rsx! { p { class: "text-error", "Error: {e}" } },
                                Some(Ok(_)) if project_txs.is_empty() => rsx! {
                                    p {
                                        style: "color: var(--text-muted); margin-bottom: 16px;",
                                        "No transactions yet — add some below."
                                    }
                                },
                                Some(Ok(_)) => rsx! {
                                    div {
                                        class: "project-tx-list",
                                        for tx in project_txs.iter() {
                                            {
                                                let tid = tx.id;
                                                let project_id = project.id;
                                                let date_str = tx.date
                                                    .map(|d| d.format("%Y-%m-%d").to_string())
                                                    .unwrap_or_else(|| "Pending".to_string());
                                                let amount_str = fmt_tx_amount(tx.amount, &tx.currency);
                                                let amount_color = if tx.amount >= Decimal::ZERO { "#16a34a" } else { "#dc2626" };
                                                let desc = tx.description.clone();
                                                let cat_name = tx.category.as_ref().map(|c| c.name.clone());
                                                rsx! {
                                                    div {
                                                        key: "{tid}",
                                                        class: "project-tx-row",
                                                        span { class: "project-row__date", "{date_str}" }
                                                        span {
                                                            class: "project-row__desc",
                                                            style: "font-size: 0.88rem; color: var(--text-secondary);",
                                                            "{desc}"
                                                        }
                                                        if let Some(cat) = cat_name {
                                                            span { class: "project-row__cat", "{cat}" }
                                                        }
                                                        span {
                                                            class: "project-row__amount",
                                                            style: "font-size: 0.88rem; color: {amount_color};",
                                                            "{amount_str}"
                                                        }
                                                        button {
                                                            onclick: move |_| async move {
                                                                let _ = remove_from_group(tid, project_id).await;
                                                                project_txs_res.restart();
                                                            },
                                                            class: "btn-ghost btn-ghost--sm",
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
                                class: "btn-toggle",
                                if show_add() { "▲ Hide add panel" } else { "▼ Add transactions" }
                            }

                            if show_add() {
                                input {
                                    r#type: "text",
                                    value: add_search(),
                                    oninput: move |e| add_search.set(e.value()),
                                    placeholder: "Search by description or date…",
                                    class: "input-std",
                                    style: "width: 100%; max-width: 400px; margin-bottom: 10px;",
                                }
                                if filtered_available.is_empty() {
                                    p {
                                        style: "color: var(--text-muted); font-size: 0.9rem;",
                                        if available_txs.is_empty() {
                                            "All transactions are already in this project."
                                        } else {
                                            "No transactions match your search."
                                        }
                                    }
                                } else {
                                    div {
                                        class: "project-available",
                                        for tx in filtered_available.iter() {
                                            {
                                                let tid = tx.id;
                                                let project_id = project.id;
                                                let date_str = tx.date
                                                    .map(|d| d.format("%Y-%m-%d").to_string())
                                                    .unwrap_or_else(|| "Pending".to_string());
                                                let amount_str = fmt_tx_amount(tx.amount, &tx.currency);
                                                let amount_color = if tx.amount >= Decimal::ZERO { "#16a34a" } else { "#dc2626" };
                                                let desc = tx.description.clone();
                                                let cat_name = tx.category.as_ref().map(|c| c.name.clone());
                                                rsx! {
                                                    div {
                                                        key: "{tid}",
                                                        class: "project-add-row",
                                                        span { class: "project-row__date", "{date_str}" }
                                                        span {
                                                            class: "project-row__desc",
                                                            style: "font-size: 0.85rem; color: var(--text-secondary);",
                                                            "{desc}"
                                                        }
                                                        if let Some(cat) = cat_name {
                                                            span { class: "project-row__cat", "{cat}" }
                                                        }
                                                        span {
                                                            class: "project-row__amount",
                                                            style: "font-size: 0.85rem; color: {amount_color};",
                                                            "{amount_str}"
                                                        }
                                                        button {
                                                            onclick: move |_| async move {
                                                                let _ = add_to_group(tid, project_id).await;
                                                                project_txs_res.restart();
                                                            },
                                                            class: "btn-primary btn-primary--sm",
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
