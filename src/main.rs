use dioxus::prelude::*;

mod api;
mod models;
mod views;

#[cfg(feature = "server")]
mod db;

// Re-exported by dioxus-server, so the version always matches the one the
// framework's own router is built from.
#[cfg(feature = "server")]
use dioxus::server::axum;

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

/// Ceiling on an incoming request body.
///
/// Server functions carry their arguments as JSON, so an uploaded photo arrives
/// base64-encoded and costs a third more than the bytes it represents. This
/// leaves room for the largest photo `api::MAX_PHOTO_BYTES` allows, plus the
/// surrounding JSON. Raised from axum's 2 MB default, which is otherwise hit
/// while the body is still being buffered — before any handler can report it.
#[cfg(feature = "server")]
const MAX_REQUEST_BODY_BYTES: usize = 24 * 1024 * 1024;

fn main() {
    #[cfg(not(feature = "server"))]
    dioxus::launch(App);

    // The server builds its own router rather than using `dioxus::launch`, for
    // two things the default one cannot express: a larger body limit, and a
    // route that returns image bytes directly instead of as JSON.
    #[cfg(feature = "server")]
    dioxus::server::serve(|| async move {
        use dioxus::server::axum::{extract::DefaultBodyLimit, routing::get, Router};
        use dioxus::server::{DioxusRouterExt, ServeConfig};

        Ok(Router::new()
            .route("/photo/{id}", get(serve_photo))
            .serve_dioxus_application(ServeConfig::default(), App)
            .layer(DefaultBodyLimit::max(MAX_REQUEST_BODY_BYTES)))
    });
}

/// Serves a recipe's photo as an ordinary image response.
///
/// Deliberately not a server function: those return JSON, which would mean
/// base64 on the wire and a `data:` URL in the page. An image URL lets the
/// browser cache it, defer it until it scrolls into view, and decode it off the
/// main thread — which is the difference between a workable page and an
/// unworkable one once photos are measured in megabytes.
#[cfg(feature = "server")]
async fn serve_photo(
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> axum::response::Response {
    use axum::http::{header, StatusCode};
    use axum::response::IntoResponse;
    use sqlx::Row;

    let Ok(pool) = db::db().await else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    let row = sqlx::query("SELECT photo, photo_mime FROM recipes WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await;

    let Ok(Some(row)) = row else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let Some(bytes) = row.get::<Option<Vec<u8>>, _>("photo") else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let mime = row
        .get::<Option<String>, _>("photo_mime")
        .unwrap_or_else(|| "image/jpeg".to_string());

    (
        [
            (header::CONTENT_TYPE, mime),
            // Cached for a year and never revalidated. Safe because callers
            // address this through `/photo/{id}?v={photo_version}`, and that
            // version changes on every write — a replaced photo is a different
            // URL rather than a stale entry under the same one.
            //
            // `public` rather than `private` so a reverse proxy in front of the
            // app can serve repeat requests without reaching a handler at all.
            // These are pictures of food, and the route carries no
            // authentication, so there is nothing here a shared cache should be
            // kept away from.
            (
                header::CACHE_CONTROL,
                "public, max-age=31536000, immutable".to_string(),
            ),
        ],
        bytes,
    )
        .into_response()
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Stylesheet { href: MAIN_CSS }
        Router::<Route> {}
    }
}
