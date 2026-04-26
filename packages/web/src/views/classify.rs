use api::models::Category;
use api::{
    classify_transaction, create_category, delete_category, get_queue_state, list_categories,
    update_category,
};
use dioxus::prelude::*;
use uuid::Uuid;
use ui::{fmt_tx_amount, TransactionQueueCard};

#[component]
pub fn Classify() -> Element {
    let mut categories_res = use_resource(list_categories);
    let mut queue_res = use_resource(get_queue_state);

    // ── Inline-edit state ─────────────────────────────────────────────────
    // Shared edit form (one at a time for both top-level and subcategories).
    let mut editing_id: Signal<Option<Uuid>> = use_signal(|| None);
    let mut edit_name = use_signal(String::new);
    let mut edit_color = use_signal(|| "#6366f1".to_string());
    let mut editing_is_top_level = use_signal(|| false);
    let mut edit_error: Signal<Option<String>> = use_signal(|| None);

    // Which parent is showing its "add subcategory" form.
    let mut adding_sub_for: Signal<Option<Uuid>> = use_signal(|| None);
    let mut new_sub_name = use_signal(String::new);
    let mut new_sub_color = use_signal(|| "#6366f1".to_string());
    let mut sub_error: Signal<Option<String>> = use_signal(|| None);

    // ── New top-level category form ────────────────────────────────────────
    let mut new_cat_name = use_signal(String::new);
    let mut cat_error: Signal<Option<String>> = use_signal(|| None);

    // ── Event handlers ─────────────────────────────────────────────────────

    let create_cat = move |_| async move {
        cat_error.set(None);
        let name = new_cat_name().trim().to_string();
        if name.is_empty() {
            cat_error.set(Some("Name cannot be empty".to_string()));
            return;
        }
        match create_category(name, String::new(), None).await {
            Ok(_) => {
                new_cat_name.set(String::new());
                categories_res.restart();
            }
            Err(e) => cat_error.set(Some(e.to_string())),
        }
    };

    let save_edit = move |_| async move {
        let id = match editing_id() {
            Some(id) => id,
            None => return,
        };
        let name = edit_name().trim().to_string();
        if name.is_empty() {
            edit_error.set(Some("Name cannot be empty".to_string()));
            return;
        }
        let color = if editing_is_top_level() { String::new() } else { edit_color() };
        match update_category(id, name, color).await {
            Ok(_) => {
                editing_id.set(None);
                edit_error.set(None);
                categories_res.restart();
                queue_res.restart();
            }
            Err(e) => edit_error.set(Some(e.to_string())),
        }
    };

    let create_sub = move |_| async move {
        let parent_id = match adding_sub_for() {
            Some(id) => id,
            None => return,
        };
        let name = new_sub_name().trim().to_string();
        if name.is_empty() {
            sub_error.set(Some("Name cannot be empty".to_string()));
            return;
        }
        match create_category(name, new_sub_color(), Some(parent_id)).await {
            Ok(_) => {
                new_sub_name.set(String::new());
                adding_sub_for.set(None);
                sub_error.set(None);
                categories_res.restart();
                queue_res.restart();
            }
            Err(e) => sub_error.set(Some(e.to_string())),
        }
    };

    let on_classify = move |(tx, cat): (api::models::Transaction, Category)| async move {
        let _ = classify_transaction(tx.id, Some(cat.id)).await;
        queue_res.restart();
    };

    // ── Derived data ───────────────────────────────────────────────────────

    let categories: Vec<Category> = categories_res()
        .and_then(|r| r.ok())
        .unwrap_or_default();

    let top_level: Vec<Category> =
        categories.iter().filter(|c| c.parent_id.is_none()).cloned().collect();

    rsx! {
        div {
            style: "padding: 32px; font-family: sans-serif; max-width: 960px;",
            h1 { style: "margin: 0 0 28px; font-size: 1.5rem; color: #111827;", "Classify" }

            div {
                style: "display: flex; gap: 32px; align-items: flex-start;",

                // ── Left column: category management ──────────────────────
                div {
                    style: "width: 340px; flex-shrink: 0;",

                    h2 { style: "margin: 0 0 16px; font-size: 1rem; color: #374151; font-weight: 700;", "Categories" }

                    // New top-level category form
                    div {
                        style: "background: #f9fafb; border: 1px solid #e5e7eb; border-radius: 12px; padding: 16px; margin-bottom: 20px;",
                        p { style: "font-size: 0.75rem; font-weight: 700; color: #6b7280; margin: 0 0 8px; text-transform: uppercase; letter-spacing: 0.05em;", "New category" }
                        div {
                            style: "display: flex; gap: 8px; align-items: flex-end;",
                            div {
                                style: "flex: 1;",
                                input {
                                    r#type: "text",
                                    value: new_cat_name(),
                                    oninput: move |e| new_cat_name.set(e.value()),
                                    placeholder: "e.g. Food",
                                    style: "width: 100%; padding: 7px 10px; border: 1px solid #d1d5db; border-radius: 8px; font-size: 0.9rem; box-sizing: border-box;",
                                }
                            }
                            button {
                                onclick: create_cat,
                                style: "padding: 7px 14px; background: #1e293b; color: #f1f5f9; border: none; border-radius: 8px; cursor: pointer; font-size: 0.85rem; font-weight: 600; white-space: nowrap;",
                                "Add"
                            }
                        }
                        if let Some(err) = cat_error() {
                            p { style: "color: #dc2626; font-size: 0.8rem; margin: 6px 0 0;", "{err}" }
                        }
                    }

                    // Existing categories tree
                    div {
                        style: "display: flex; flex-direction: column; gap: 10px;",

                        for parent in top_level.iter() {
                            {
                                let parent_id = parent.id;
                                let parent_name = parent.name.clone();
                                let parent_color = parent.color.clone();
                                let subcats: Vec<Category> = categories
                                    .iter()
                                    .filter(|c| c.parent_id == Some(parent_id))
                                    .cloned()
                                    .collect();
                                let is_editing_parent = editing_id() == Some(parent_id);
                                let is_adding_sub = adding_sub_for() == Some(parent_id);

                                rsx! {
                                    div {
                                        key: "{parent_id}",
                                        style: "background: #fff; border: 1px solid #e5e7eb; border-radius: 10px; overflow: hidden;",

                                        // ── Parent row ────────────────────
                                        if is_editing_parent {
                                            // Inline edit form
                                            div {
                                                style: "padding: 10px 12px; background: #f9fafb; border-bottom: 1px solid #e5e7eb;",
                                                // Row 1: text input (no color for top-level categories)
                                                div {
                                                    style: "display: flex; gap: 8px; align-items: center;",
                                                    input {
                                                        r#type: "text",
                                                        value: edit_name(),
                                                        oninput: move |e| edit_name.set(e.value()),
                                                        style: "flex: 1; min-width: 0; padding: 5px 8px; border: 1px solid #d1d5db; border-radius: 6px; font-size: 0.85rem; box-sizing: border-box;",
                                                    }
                                                }
                                                // Row 2: action buttons
                                                div {
                                                    style: "display: flex; gap: 6px; justify-content: flex-end; margin-top: 6px;",
                                                    button {
                                                        onclick: save_edit,
                                                        style: "padding: 5px 14px; background: #1e293b; color: #f1f5f9; border: none; border-radius: 6px; cursor: pointer; font-size: 0.8rem; font-weight: 600;",
                                                        "Save"
                                                    }
                                                    button {
                                                        onclick: move |_| { editing_id.set(None); edit_error.set(None); },
                                                        class: "btn-ghost",
                                                        style: "padding: 5px 12px; background: transparent; color: #6b7280; border: 1px solid #d1d5db; border-radius: 6px; cursor: pointer; font-size: 0.8rem;",
                                                        "Cancel"
                                                    }
                                                }
                                                if let Some(err) = edit_error() {
                                                    p { style: "color: #dc2626; font-size: 0.78rem; margin: 4px 0 0;", "{err}" }
                                                }
                                            }
                                        } else {
                                            // Display row
                                            div {
                                                style: "display: flex; align-items: center; gap: 8px; padding: 10px 12px;",
                                                span {
                                                    style: "flex: 1; font-size: 0.9rem; font-weight: 600; color: #111827;",
                                                    "{parent_name}"
                                                }
                                                button {
                                                     onclick: move |_| {
                                                        editing_id.set(Some(parent_id));
                                                        edit_name.set(parent_name.clone());
                                                        edit_color.set(parent_color.clone());
                                                        editing_is_top_level.set(true);
                                                        edit_error.set(None);
                                                    },
                                                    class: "btn-ghost",
                                                    style: "padding: 3px 10px; background: transparent; color: #6b7280; border: 1px solid #e5e7eb; border-radius: 6px; cursor: pointer; font-size: 0.75rem;",
                                                    "Edit"
                                                }
                                                button {
                                                    onclick: move |_| async move {
                                                        let _ = delete_category(parent_id).await;
                                                        categories_res.restart();
                                                        queue_res.restart();
                                                    },
                                                    class: "btn-danger",
                                                    style: "padding: 3px 10px; background: transparent; color: #dc2626; border: 1px solid #fecaca; border-radius: 6px; cursor: pointer; font-size: 0.75rem;",
                                                    "Delete"
                                                }
                                            }
                                        }

                                        // ── Subcategory rows ──────────────
                                        for sub in subcats.iter() {
                                            {
                                                let sub_id = sub.id;
                                                let sub_name = sub.name.clone();
                                                let sub_color = sub.color.clone();
                                                let is_editing_sub = editing_id() == Some(sub_id);

                                                rsx! {
                                                    div {
                                                        key: "{sub_id}",
                                                        style: "border-top: 1px solid #f3f4f6;",

                                                        if is_editing_sub {
                                                            div {
                                                                style: "padding: 8px 12px 8px 28px; background: #f9fafb;",
                                                                // Row 1: text input + color
                                                                div {
                                                                    style: "display: flex; gap: 8px; align-items: center;",
                                                                    input {
                                                                        r#type: "text",
                                                                        value: edit_name(),
                                                                        oninput: move |e| edit_name.set(e.value()),
                                                                        style: "flex: 1; min-width: 0; padding: 5px 8px; border: 1px solid #d1d5db; border-radius: 6px; font-size: 0.82rem; box-sizing: border-box;",
                                                                    }
                                                                    input {
                                                                        r#type: "color",
                                                                        value: edit_color(),
                                                                        oninput: move |e| edit_color.set(e.value()),
                                                                        style: "width: 32px; height: 32px; flex-shrink: 0; padding: 2px; border: 1px solid #d1d5db; border-radius: 6px; cursor: pointer; appearance: none; -webkit-appearance: none;",
                                                                    }
                                                                }
                                                                // Row 2: action buttons
                                                                div {
                                                                    style: "display: flex; gap: 6px; justify-content: flex-end; margin-top: 6px;",
                                                                    button {
                                                                        onclick: save_edit,
                                                                        style: "padding: 5px 14px; background: #1e293b; color: #f1f5f9; border: none; border-radius: 6px; cursor: pointer; font-size: 0.78rem; font-weight: 600;",
                                                                        "Save"
                                                                    }
                                                                    button {
                                                                        onclick: move |_| { editing_id.set(None); edit_error.set(None); },
                                                                        class: "btn-ghost",
                                                                        style: "padding: 5px 12px; background: transparent; color: #6b7280; border: 1px solid #d1d5db; border-radius: 6px; cursor: pointer; font-size: 0.78rem;",
                                                                        "Cancel"
                                                                    }
                                                                }
                                                                if let Some(err) = edit_error() {
                                                                    p { style: "color: #dc2626; font-size: 0.75rem; margin: 4px 0 0;", "{err}" }
                                                                }
                                                            }
                                                        } else {
                                                            div {
                                                                style: "display: flex; align-items: center; gap: 8px; padding: 8px 12px 8px 28px;",
                                                                span {
                                                                    style: "width: 10px; height: 10px; border-radius: 50%; background: {sub_color}; flex-shrink: 0;",
                                                                }
                                                                span {
                                                                    style: "flex: 1; font-size: 0.85rem; color: #374151;",
                                                                    "{sub_name}"
                                                                }
                                                                button {
                                                                    onclick: move |_| {
                                                                        editing_id.set(Some(sub_id));
                                                                        edit_name.set(sub_name.clone());
                                                                        edit_color.set(sub_color.clone());
                                                                        editing_is_top_level.set(false);
                                                                        edit_error.set(None);
                                                                    },
                                                                    class: "btn-ghost",
                                                                    style: "padding: 2px 8px; background: transparent; color: #6b7280; border: 1px solid #e5e7eb; border-radius: 6px; cursor: pointer; font-size: 0.72rem;",
                                                                    "Edit"
                                                                }
                                                                button {
                                                                    onclick: move |_| async move {
                                                                        let _ = delete_category(sub_id).await;
                                                                        categories_res.restart();
                                                                        queue_res.restart();
                                                                    },
                                                                    class: "btn-danger",
                                                                    style: "padding: 2px 8px; background: transparent; color: #dc2626; border: 1px solid #fecaca; border-radius: 6px; cursor: pointer; font-size: 0.72rem;",
                                                                    "Delete"
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        // ── Add subcategory ───────────────
                                        div {
                                            style: "border-top: 1px solid #f3f4f6; padding: 6px 12px 6px 28px;",

                                            if is_adding_sub {
                                                div {
                                                    // Row 1: text input + color
                                                    div {
                                                        style: "display: flex; gap: 8px; align-items: center; padding: 4px 0;",
                                                        input {
                                                            r#type: "text",
                                                            value: new_sub_name(),
                                                            oninput: move |e| new_sub_name.set(e.value()),
                                                            placeholder: "Subcategory name",
                                                            style: "flex: 1; min-width: 0; padding: 5px 8px; border: 1px solid #d1d5db; border-radius: 6px; font-size: 0.82rem; box-sizing: border-box;",
                                                        }
                                                        input {
                                                            r#type: "color",
                                                            value: new_sub_color(),
                                                            oninput: move |e| new_sub_color.set(e.value()),
                                                            style: "width: 32px; height: 32px; flex-shrink: 0; padding: 2px; border: 1px solid #d1d5db; border-radius: 6px; cursor: pointer; appearance: none; -webkit-appearance: none;",
                                                        }
                                                    }
                                                    // Row 2: action buttons
                                                    div {
                                                        style: "display: flex; gap: 6px; justify-content: flex-end; margin-top: 2px;",
                                                        button {
                                                            onclick: create_sub,
                                                            style: "padding: 5px 14px; background: #1e293b; color: #f1f5f9; border: none; border-radius: 6px; cursor: pointer; font-size: 0.78rem; font-weight: 600;",
                                                            "Add"
                                                        }
                                                        button {
                                                            onclick: move |_| { adding_sub_for.set(None); sub_error.set(None); },
                                                            class: "btn-ghost",
                                                            style: "padding: 5px 12px; background: transparent; color: #6b7280; border: 1px solid #d1d5db; border-radius: 6px; cursor: pointer; font-size: 0.78rem;",
                                                            "Cancel"
                                                        }
                                                    }
                                                    if let Some(err) = sub_error() {
                                                        p { style: "color: #dc2626; font-size: 0.75rem; margin: 2px 0 0;", "{err}" }
                                                    }
                                                }
                                            } else {
                                                button {
                                                    onclick: move |_| {
                                                        adding_sub_for.set(Some(parent_id));
                                                        new_sub_name.set(String::new());
                                                        sub_error.set(None);
                                                    },
                                                    class: "btn-ghost",
                                                    style: "background: transparent; border: none; color: #6b7280; font-size: 0.78rem; cursor: pointer; padding: 4px 0;",
                                                    "+ Add subcategory"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // ── Right column: classify queue ───────────────────────────
                div {
                    style: "flex: 1; min-width: 0;",

                    match queue_res() {
                        None => rsx! { p { "Loading…" } },
                        Some(Err(e)) => rsx! { p { style: "color: red;", "Error: {e}" } },
                        Some(Ok(state)) => {
                            if state.remaining == 0 {
                                rsx! {
                                    p {
                                        style: "color: #6b7280; padding: 16px 0;",
                                        "All transactions classified."
                                    }
                                }
                            } else {
                                let remaining = state.remaining;
                                let plural = if remaining == 1 { "" } else { "s" };
                                rsx! {
                                    p {
                                        style: "font-size: 0.85rem; color: #6b7280; margin: 0 0 16px;",
                                        "{remaining} transaction{plural} remaining"
                                    }
                                    if let Some(tx) = state.next {
                                        TransactionQueueCard {
                                            transaction: tx,
                                            categories: categories.clone(),
                                            on_classify: EventHandler::new(on_classify),
                                        }
                                    }
                // ── Upcoming preview ──────────────────
                                    if !state.upcoming.is_empty() {
                                        div {
                                            style: "margin-top: 12px; display: flex; flex-direction: column; gap: 1px; opacity: 0.45; pointer-events: none; user-select: none;",
                                            for tx in state.upcoming.iter() {
                                                {
                                                    use rust_decimal::Decimal;
                                                    let date_str = tx.date
                                                        .map(|d| d.format("%Y-%m-%d").to_string())
                                                        .unwrap_or_else(|| "Pending".to_string());
                                                    let amount_color = if tx.amount >= Decimal::ZERO { "#16a34a" } else { "#dc2626" };
                                                    let amount_str = fmt_tx_amount(tx.amount, &tx.currency);
                                                    rsx! {
                                                        div {
                                                            key: "{tx.id}",
                                                            style: "display: flex; align-items: center; gap: 12px; padding: 10px 16px; background: #fff; border: 1px solid #e5e7eb; border-radius: 10px;",
                                                            span {
                                                                style: "min-width: 86px; font-size: 0.78rem; color: #9ca3af;",
                                                                "{date_str}"
                                                            }
                                                            span {
                                                                style: "flex: 1; font-size: 0.88rem; color: #374151; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                                                                "{tx.description}"
                                                            }
                                                            span {
                                                                style: "font-size: 0.88rem; font-weight: 600; color: {amount_color};",
                                                                "{amount_str}"
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
    }
}
