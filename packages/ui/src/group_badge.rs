use api::models::Group;
use dioxus::prelude::*;

/// A small pill showing a group name.
#[component]
pub fn GroupBadge(group: Group) -> Element {
    rsx! {
        span {
            style: "background-color: #dbeafe; color: #1d4ed8; padding: 2px 10px; border-radius: 999px; font-size: 0.75rem; font-weight: 600;",
            "{group.name}"
        }
    }
}
