use dioxus::prelude::*;

#[component]
pub fn Pantry() -> Element {
    rsx! {
        section {
            h2 { "Pantry" }
            p { class: "muted",
                "Not built yet. The "
                code { "pantry" }
                " table is ready to track what is already at home."
            }
        }
    }
}
