use crate::Route;
use dioxus::prelude::*;

/// Shared shell around every page. The links sit at the bottom on narrow
/// screens so they stay reachable one-handed on a phone in the kitchen.
#[component]
pub fn Navbar() -> Element {
    rsx! {
        header { id: "app-header",
            h1 { "Food Planner" }
        }

        main { id: "content",
            Outlet::<Route> {}
        }

        nav { id: "navbar",
            Link { to: Route::Recipes {}, active_class: "active", "Recipes" }
            Link { to: Route::Planner {}, active_class: "active", "Plan" }
            Link { to: Route::Shopping {}, active_class: "active", "Shop" }
            Link { to: Route::Pantry {}, active_class: "active", "Pantry" }
        }
    }
}
