use api::models::Group;
use dioxus::prelude::*;

/// A small pill showing a group name.
#[component]
pub fn GroupBadge(group: Group) -> Element {
    rsx! {
        span { class: "badge badge--group", "{group.name}" }
    }
}
