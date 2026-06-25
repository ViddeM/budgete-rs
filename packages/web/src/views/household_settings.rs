use api::{get_household_info, leave_household, regenerate_invite_code};
use dioxus::prelude::*;

#[component]
pub fn HouseholdSettings() -> Element {
    let info = use_server_future(get_household_info)?;
    let mut copied = use_signal(|| false);
    let mut regen_loading = use_signal(|| false);
    let mut regen_error = use_signal(|| Option::<String>::None);
    let mut override_code = use_signal(|| Option::<String>::None);
    let mut leave_loading = use_signal(|| false);
    let mut leave_error = use_signal(|| Option::<String>::None);
    let nav = use_navigator();

    let current_code = move || {
        override_code().clone().or_else(|| {
            info().and_then(|r| r.ok()).map(|h| h.invite_code)
        })
    };

    let on_copy = move |_| {
        if let Some(code) = current_code() {
            let _ = dioxus::document::eval(&format!(
                "navigator.clipboard.writeText('{code}').catch(()=>{{}});"
            ));
            copied.set(true);
        }
    };

    let on_regen = move |_| {
        regen_loading.set(true);
        regen_error.set(None);
        copied.set(false);
        spawn(async move {
            match regenerate_invite_code().await {
                Ok(new_code) => {
                    override_code.set(Some(new_code));
                    regen_loading.set(false);
                }
                Err(e) => {
                    regen_error.set(Some(e.to_string()));
                    regen_loading.set(false);
                }
            }
        });
    };

    let on_leave = move |_| {
        leave_loading.set(true);
        leave_error.set(None);
        spawn(async move {
            match leave_household().await {
                Ok(()) => { nav.push("/household/setup"); }
                Err(e) => {
                    leave_error.set(Some(e.to_string()));
                    leave_loading.set(false);
                }
            }
        });
    };

    rsx! {
        div {
            class: "view view--narrow",
            h1 { class: "view__title", "Household" }

            match info() {
                None => rsx! { p { class: "text-muted", "Loading…" } },
                Some(Err(e)) => rsx! { p { class: "form-error", "{e}" } },
                Some(Ok(household)) => rsx! {
                    div { class: "settings-section",
                        h2 { class: "settings-section__title", "{household.name}" }
                    }

                    div { class: "settings-section",
                        h3 { class: "settings-section__label", "Invite code" }
                        p { class: "settings-section__hint",
                            "Share this code with people you want to add to your household."
                        }
                        div { class: "invite-code-row",
                            span { class: "invite-code", "{current_code().unwrap_or_default()}" }
                            button {
                                class: "btn-ghost",
                                onclick: on_copy,
                                if copied() { "Copied!" } else { "Copy" }
                            }
                        }
                        if let Some(err) = regen_error() {
                            p { class: "form-error", "{err}" }
                        }
                        button {
                            class: "btn-ghost btn-ghost--sm",
                            onclick: on_regen,
                            disabled: regen_loading(),
                            if regen_loading() { "Regenerating…" } else { "Regenerate code" }
                        }
                    }

                    div { class: "settings-section",
                        h3 { class: "settings-section__label", "Members" }
                        ul { class: "member-list",
                            for member in household.members {
                                li { class: "member-item",
                                    span { class: "member-avatar",
                                        {member.name.as_deref()
                                            .and_then(|n| n.chars().next())
                                            .map(|c| c.to_uppercase().to_string())
                                            .unwrap_or_else(|| "?".to_string())}
                                    }
                                    div { class: "member-info",
                                        span { class: "member-name",
                                            {member.name.as_deref().unwrap_or("Unknown")}
                                        }
                                        if let Some(email) = member.email {
                                            span { class: "member-email", "{email}" }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    div { class: "settings-section settings-section--danger",
                        h3 { class: "settings-section__label", "Leave household" }
                        p { class: "settings-section__hint",
                            "You will be taken to the setup screen to create or join a different household."
                        }
                        if let Some(err) = leave_error() {
                            p { class: "form-error", "{err}" }
                        }
                        button {
                            class: "btn-danger",
                            onclick: on_leave,
                            disabled: leave_loading(),
                            if leave_loading() { "Leaving…" } else { "Leave household" }
                        }
                    }
                },
            }
        }
    }
}
