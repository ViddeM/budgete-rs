use api::models::Category;
use api::{
    classify_transaction, create_category, delete_category, get_queue_state, list_categories,
    update_category,
};
use dioxus::prelude::*;
use ui::{fmt_date, fmt_tx_amount, tx_amount_color, TransactionQueueCard};
use uuid::Uuid;

#[component]
pub fn Classify() -> Element {
    let mut categories_res = use_resource(list_categories);
    let mut queue_res = use_resource(get_queue_state);

    // ── Inline-edit state ─────────────────────────────────────────────────
    let mut editing_id: Signal<Option<Uuid>> = use_signal(|| None);
    let mut edit_name = use_signal(String::new);
    let mut edit_color = use_signal(|| "#6366f1".to_string());
    let mut editing_is_top_level = use_signal(|| false);
    let mut edit_error: Signal<Option<String>> = use_signal(|| None);

    let mut adding_sub_for: Signal<Option<Uuid>> = use_signal(|| None);
    let mut new_sub_name = use_signal(String::new);
    let mut new_sub_color = use_signal(|| "#6366f1".to_string());
    let mut new_sub_ignored = use_signal(|| false);
    let mut sub_error: Signal<Option<String>> = use_signal(|| None);

    // ── New top-level category form ────────────────────────────────────────
    let mut new_cat_name = use_signal(String::new);
    let mut new_cat_color = use_signal(|| "#6366f1".to_string());
    let mut new_cat_ignored = use_signal(|| false);
    let mut cat_error: Signal<Option<String>> = use_signal(|| None);

    // ── Undo state ────────────────────────────────────────────────────────
    let mut last_classified: Signal<Option<Uuid>> = use_signal(|| None);

    let mut edit_ignored = use_signal(|| false);

    // ── Event handlers ─────────────────────────────────────────────────────

    let create_cat = move |_| async move {
        cat_error.set(None);
        let name = new_cat_name().trim().to_string();
        if name.is_empty() {
            cat_error.set(Some("Name cannot be empty".to_string()));
            return;
        }
        match create_category(name, new_cat_color(), None, new_cat_ignored()).await {
            Ok(_) => {
                new_cat_name.set(String::new());
                new_cat_color.set("#6366f1".to_string());
                new_cat_ignored.set(false);
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
        let color = edit_color();
        match update_category(id, name, color, edit_ignored()).await {
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
        match create_category(name, new_sub_color(), Some(parent_id), new_sub_ignored()).await {
            Ok(_) => {
                new_sub_name.set(String::new());
                new_sub_ignored.set(false);
                adding_sub_for.set(None);
                sub_error.set(None);
                categories_res.restart();
                queue_res.restart();
            }
            Err(e) => sub_error.set(Some(e.to_string())),
        }
    };

    let on_classify = move |(tx, cat): (api::models::Transaction, Category)| async move {
        let tx_id = tx.id;
        let _ = classify_transaction(tx_id, Some(cat.id)).await;
        last_classified.set(Some(tx_id));
        queue_res.restart();
    };

    let on_undo = move |_| async move {
        if let Some(tx_id) = last_classified() {
            let _ = classify_transaction(tx_id, None).await;
            last_classified.set(None);
            queue_res.restart();
        }
    };

    // ── Derived data ───────────────────────────────────────────────────────

    let categories: Vec<Category> = categories_res().and_then(|r| r.ok()).unwrap_or_default();

    let top_level: Vec<Category> = categories
        .iter()
        .filter(|c| c.parent_id.is_none())
        .cloned()
        .collect();

    rsx! {
        div {
            class: "view view--wide",
            h1 { class: "view__title", style: "margin-bottom: 28px;", "Classify" }

            div {
                class: "two-col",

                // ── Left column: category management ──────────────────────
                div {
                    class: "classify-col-left",

                    h2 { class: "view__section-title", style: "margin-bottom: 16px; font-weight: 700;", "Categories" }

                    // New top-level category form
                    div {
                        class: "form-card",
                        p { class: "form-card__title", "New category" }
                        div {
                            class: "form-row",
                            div {
                                style: "flex: 1;",
                                input {
                                    r#type: "text",
                                    value: new_cat_name(),
                                    oninput: move |e| new_cat_name.set(e.value()),
                                    placeholder: "e.g. Food",
                                    class: "input-std input-std--full",
                                }
                            }
                            input {
                                r#type: "color",
                                value: new_cat_color(),
                                oninput: move |e| new_cat_color.set(e.value()),
                                class: "color-input color-input--lg",
                            }
                            button {
                                onclick: create_cat,
                                class: "btn-primary",
                                "Add"
                            }
                        }
                        label {
                            class: "checkbox-row",
                            input {
                                r#type: "checkbox",
                                checked: new_cat_ignored(),
                                oninput: move |e| new_cat_ignored.set(e.checked()),
                            }
                            span { "Ignore in totals" }
                        }
                        if let Some(err) = cat_error() {
                            p { class: "form-error", "{err}" }
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
                                let parent_ignored = parent.ignored;
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
                                        class: "cat-node",

                                        // ── Parent row ────────────────────
                                        if is_editing_parent {
                                            div {
                                                class: "cat-node__parent-edit",
                                                div {
                                                    class: "form-row form-row--center",
                                                    input {
                                                        r#type: "text",
                                                        value: edit_name(),
                                                        oninput: move |e| edit_name.set(e.value()),
                                                        class: "input-std input-std--compact",
                                                        style: "flex: 1; min-width: 0;",
                                                    }
                                                    input {
                                                        r#type: "color",
                                                        value: edit_color(),
                                                        oninput: move |e| edit_color.set(e.value()),
                                                        class: "color-input color-input--md",
                                                    }
                                                }
                                                label {
                                                    class: "checkbox-row checkbox-row--sm",
                                                    input {
                                                        r#type: "checkbox",
                                                        checked: edit_ignored(),
                                                        oninput: move |e| edit_ignored.set(e.checked()),
                                                    }
                                                    span { "Ignore in totals" }
                                                }
                                                div {
                                                    class: "btn-actions",
                                                    button { onclick: save_edit, class: "btn-primary btn-primary--sm", "Save" }
                                                    button {
                                                        onclick: move |_| { editing_id.set(None); edit_error.set(None); },
                                                        class: "btn-ghost",
                                                        "Cancel"
                                                    }
                                                }
                                                if let Some(err) = edit_error() {
                                                    p { class: "form-error form-error--sm", "{err}" }
                                                }
                                            }
                                        } else {
                                            div {
                                                class: "cat-node__parent-display",
                                                span {
                                                    class: "color-dot color-dot--lg",
                                                    style: "background: {parent_color};",
                                                }
                                                span {
                                                    style: "flex: 1; font-size: 0.9rem; font-weight: 600; color: var(--text-primary);",
                                                    "{parent_name}"
                                                }
                                                if parent_ignored {
                                                    span { class: "ignored-badge", "ignored" }
                                                }
                                                button {
                                                    onclick: move |_| {
                                                        editing_id.set(Some(parent_id));
                                                        edit_name.set(parent_name.clone());
                                                        edit_color.set(parent_color.clone());
                                                        edit_ignored.set(parent_ignored);
                                                        editing_is_top_level.set(true);
                                                        edit_error.set(None);
                                                    },
                                                    class: "btn-ghost btn-ghost--sm",
                                                    "Edit"
                                                }
                                                button {
                                                    onclick: move |_| async move {
                                                        let _ = delete_category(parent_id).await;
                                                        categories_res.restart();
                                                        queue_res.restart();
                                                    },
                                                    class: "btn-danger",
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
                                                let sub_ignored = sub.ignored;
                                                let is_editing_sub = editing_id() == Some(sub_id);

                                                rsx! {
                                                    div {
                                                        key: "{sub_id}",
                                                        class: "cat-node__sub-row",

                                                        if is_editing_sub {
                                                            div {
                                                                class: "cat-node__sub-edit",
                                                                div {
                                                                    class: "form-row form-row--center",
                                                                    input {
                                                                        r#type: "text",
                                                                        value: edit_name(),
                                                                        oninput: move |e| edit_name.set(e.value()),
                                                                        class: "input-std input-std--compact",
                                                                        style: "flex: 1; min-width: 0;",
                                                                    }
                                                                    input {
                                                                        r#type: "color",
                                                                        value: edit_color(),
                                                                        oninput: move |e| edit_color.set(e.value()),
                                                                        class: "color-input color-input--md",
                                                                    }
                                                                }
                                                                label {
                                                                    class: "checkbox-row checkbox-row--sm",
                                                                    input {
                                                                        r#type: "checkbox",
                                                                        checked: edit_ignored(),
                                                                        oninput: move |e| edit_ignored.set(e.checked()),
                                                                    }
                                                                    span { "Ignore in totals" }
                                                                }
                                                                div {
                                                                    class: "btn-actions",
                                                                    button { onclick: save_edit, class: "btn-primary btn-primary--xs", "Save" }
                                                                    button {
                                                                        onclick: move |_| { editing_id.set(None); edit_error.set(None); },
                                                                        class: "btn-ghost",
                                                                        "Cancel"
                                                                    }
                                                                }
                                                                if let Some(err) = edit_error() {
                                                                    p { class: "form-error form-error--xs", "{err}" }
                                                                }
                                                            }
                                                        } else {
                                                            div {
                                                                class: "cat-node__sub-display",
                                                                span {
                                                                    class: "color-dot color-dot--md",
                                                                    style: "background: {sub_color};",
                                                                }
                                                                span {
                                                                    style: "flex: 1; font-size: 0.85rem; color: var(--text-secondary);",
                                                                    "{sub_name}"
                                                                }
                                                                if sub_ignored {
                                                                    span { class: "ignored-badge", "ignored" }
                                                                }
                                                                button {
                                                                    onclick: move |_| {
                                                                        editing_id.set(Some(sub_id));
                                                                        edit_name.set(sub_name.clone());
                                                                        edit_color.set(sub_color.clone());
                                                                        edit_ignored.set(sub_ignored);
                                                                        editing_is_top_level.set(false);
                                                                        edit_error.set(None);
                                                                    },
                                                                    class: "btn-ghost btn-ghost--xs",
                                                                    "Edit"
                                                                }
                                                                button {
                                                                    onclick: move |_| async move {
                                                                        let _ = delete_category(sub_id).await;
                                                                        categories_res.restart();
                                                                        queue_res.restart();
                                                                    },
                                                                    class: "btn-danger btn-danger--xs",
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
                                            class: "cat-node__add-sub",

                                            if is_adding_sub {
                                                div {
                                                    div {
                                                        class: "form-row form-row--center",
                                                        style: "padding: 4px 0;",
                                                        input {
                                                            r#type: "text",
                                                            value: new_sub_name(),
                                                            oninput: move |e| new_sub_name.set(e.value()),
                                                            placeholder: "Subcategory name",
                                                            class: "input-std input-std--compact",
                                                            style: "flex: 1; min-width: 0;",
                                                        }
                                                        input {
                                                            r#type: "color",
                                                            value: new_sub_color(),
                                                            oninput: move |e| new_sub_color.set(e.value()),
                                                            class: "color-input color-input--md",
                                                        }
                                                    }
                                                    label {
                                                        class: "checkbox-row checkbox-row--sm",
                                                        style: "margin: 4px 0 2px;",
                                                        input {
                                                            r#type: "checkbox",
                                                            checked: new_sub_ignored(),
                                                            oninput: move |e| new_sub_ignored.set(e.checked()),
                                                        }
                                                        span { "Ignore in totals" }
                                                    }
                                                    div {
                                                        class: "btn-actions",
                                                        style: "margin-top: 2px;",
                                                        button { onclick: create_sub, class: "btn-primary btn-primary--xs", "Add" }
                                                        button {
                                                            onclick: move |_| { adding_sub_for.set(None); sub_error.set(None); },
                                                            class: "btn-ghost",
                                                            "Cancel"
                                                        }
                                                    }
                                                    if let Some(err) = sub_error() {
                                                        p { class: "form-error form-error--xs", "{err}" }
                                                    }
                                                }
                                            } else {
                                                button {
                                                    onclick: move |_| {
                                                        adding_sub_for.set(Some(parent_id));
                                                        new_sub_name.set(String::new());
                                                        sub_error.set(None);
                                                    },
                                                    class: "btn-text",
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
                    class: "two-col__right",

                    if last_classified().is_some() {
                        div {
                            style: "margin-bottom: 12px;",
                            button { onclick: on_undo, class: "btn-undo", "↩ Undo" }
                        }
                    }

                    match queue_res() {
                        None => rsx! { p { "Loading…" } },
                        Some(Err(e)) => rsx! { p { class: "text-error", "Error: {e}" } },
                        Some(Ok(state)) => {
                            if state.remaining == 0 {
                                rsx! {
                                    p {
                                        style: "color: var(--text-muted); padding: 16px 0;",
                                        "All transactions classified."
                                    }
                                }
                            } else {
                                let remaining = state.remaining;
                                let plural = if remaining == 1 { "" } else { "s" };
                                rsx! {
                                    p {
                                        class: "queue-remaining",
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
                                            class: "upcoming-preview",
                                            for tx in state.upcoming.iter() {
                                                {
                                                    let date_str = fmt_date(tx.date);
                                                    let amount_color = tx_amount_color(tx.amount);
                                                    let amount_str = fmt_tx_amount(tx.amount, &tx.currency);
                                                    rsx! {
                                                        div {
                                                            key: "{tx.id}",
                                                            class: "upcoming-preview__row",
                                                            span { class: "upcoming-preview__date", "{date_str}" }
                                                            span { class: "upcoming-preview__desc", "{tx.description}" }
                                                            span {
                                                                class: "upcoming-preview__amount",
                                                                style: "color: {amount_color};",
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
