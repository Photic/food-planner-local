use dioxus::prelude::*;

mod api;
mod models;
mod views;

#[cfg(feature = "server")]
mod db;

use views::{Navbar, Pantry, Planner, Recipes, Shopping};

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(Navbar)]
    #[route("/")]
    Recipes {},
    #[route("/planner")]
    Planner {},
    #[route("/shopping")]
    Shopping {},
    #[route("/pantry")]
    Pantry {},
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Stylesheet { href: MAIN_CSS }
        Router::<Route> {}
    }
}
