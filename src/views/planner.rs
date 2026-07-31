use dioxus::prelude::*;

#[component]
pub fn Planner() -> Element {
    rsx! {
        section {
            h2 { "Weekly plan" }
            p { class: "muted",
                "Not built yet. The "
                code { "meal_plan" }
                " table is ready: a recipe per day and meal slot."
            }
        }
    }
}
