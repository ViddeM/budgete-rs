use api::{create_household, join_household};
use dioxus::prelude::*;

#[component]
pub fn HouseholdSetup() -> Element {
    let mut create_name = use_signal(String::new);
    let mut join_code = use_signal(String::new);
    let mut error = use_signal(|| Option::<String>::None);
    let mut loading = use_signal(|| false);
    let nav = use_navigator();

    let on_create = move |_| {
        let name = create_name().trim().to_string();
        if name.is_empty() {
            error.set(Some("Please enter a household name.".into()));
            return;
        }
        loading.set(true);
        error.set(None);
        spawn(async move {
            match create_household(name).await {
                Ok(()) => {
                    nav.push("/");
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                    loading.set(false);
                }
            }
        });
    };

    let on_join = move |_| {
        let code = join_code().trim().to_uppercase();
        if code.is_empty() {
            error.set(Some("Please enter an invite code.".into()));
            return;
        }
        loading.set(true);
        error.set(None);
        spawn(async move {
            match join_household(code).await {
                Ok(()) => {
                    nav.push("/");
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                    loading.set(false);
                }
            }
        });
    };

    rsx! {
        div {
            class: "setup-page",
            div {
                class: "setup-container",
                h1 { class: "setup-title", "Set up your household" }
                p { class: "setup-sub", "Households let you share budgets with your family or housemates." }

                if let Some(err) = error() {
                    p { class: "form-error setup-error", "{err}" }
                }

                div { class: "setup-cards",
                    div { class: "setup-card",
                        h2 { class: "setup-card__title", "Create a new household" }
                        p { class: "setup-card__desc", "Start fresh — you'll get an invite code to share with others." }
                        div { class: "form-field",
                            label { class: "form-label", "Household name" }
                            input {
                                class: "input-std input-std--full input-std--mb",
                                r#type: "text",
                                placeholder: "e.g. The Smiths",
                                value: create_name(),
                                oninput: move |e| create_name.set(e.value()),
                                disabled: loading(),
                            }
                        }
                        button {
                            class: "btn-primary btn-primary--full",
                            onclick: on_create,
                            disabled: loading(),
                            if loading() { "Creating…" } else { "Create household" }
                        }
                    }

                    div { class: "setup-divider",
                        span { "or" }
                    }

                    div { class: "setup-card",
                        h2 { class: "setup-card__title", "Join an existing household" }
                        p { class: "setup-card__desc", "Enter the invite code shared with you." }
                        div { class: "form-field",
                            label { class: "form-label", "Invite code" }
                            input {
                                class: "input-std input-std--full input-std--mb",
                                r#type: "text",
                                placeholder: "e.g. A3F2-8BD1",
                                value: join_code(),
                                oninput: move |e| join_code.set(e.value()),
                                disabled: loading(),
                            }
                        }
                        button {
                            class: "btn-primary btn-primary--full",
                            onclick: on_join,
                            disabled: loading(),
                            if loading() { "Joining…" } else { "Join household" }
                        }
                    }
                }
            }
        }
    }
}
