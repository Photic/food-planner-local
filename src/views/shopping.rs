use dioxus::prelude::*;

#[component]
pub fn Shopping() -> Element {
    rsx! {
        section {
            h2 { "Shopping list" }
            p { class: "muted",
                "Not built yet. This will roll up the ingredients of everything on the
                 plan, subtract what the pantry already holds, and keep the tick-boxes
                 in the "
                code { "shopping_list_state" }
                " table."
            }
        }
    }
}
