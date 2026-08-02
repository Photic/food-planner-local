//! Server functions: the boundary between the browser and SQLite.
//!
//! Each one compiles twice. On the server the body runs against the database;
//! on the client the macro replaces it with an HTTP call to the same path.

use crate::models::*;
use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::db::db;
#[cfg(feature = "server")]
use base64::{engine::general_purpose::STANDARD, Engine as _};
#[cfg(feature = "server")]
use sqlx::Row;

/// Ceiling on a stored photo, in bytes.
///
/// Must stay within the request-body limit set in `main`, which has to allow
/// this plus the third that base64 adds. Exceeding the transport limit is not
/// an error that can be reported — the body is refused while it is still being
/// buffered, and the server function panics — so this has to reject first for
/// the caller to get an explanation rather than a crash.
///
/// Enforced here as well as in the browser, because the browser-side check is
/// only a courtesy: the endpoint is reachable without it.
#[cfg(feature = "server")]
const MAX_PHOTO_BYTES: usize = 15 * 1024 * 1024;

#[get("/api/recipes")]
pub async fn list_recipes() -> Result<Vec<Recipe>, ServerFnError> {
    use std::collections::HashMap;

    let pool = db().await.map_err(ServerFnError::new)?;

    // `photo_version` rather than `photo`: it says both whether an image exists
    // and which one, and selecting the blob would pull every image out of the
    // database only to discard it here.
    let recipe_rows = sqlx::query(
        "SELECT id, name, instructions, servings, photo_version \
         FROM recipes ORDER BY name COLLATE NOCASE",
    )
    .fetch_all(pool)
    .await
    .map_err(ServerFnError::new)?;

    let mut recipes: Vec<Recipe> = recipe_rows
        .into_iter()
        .map(|row| Recipe {
            id: row.get("id"),
            name: row.get("name"),
            instructions: row.get("instructions"),
            servings: row.get("servings"),
            ingredients: Vec::new(),
            photo_version: row.get("photo_version"),
        })
        .collect();

    // Fetched in one go and grouped in memory rather than one query per recipe.
    let ingredient_rows =
        sqlx::query("SELECT recipe_id, name, quantity, unit FROM ingredients ORDER BY id")
            .fetch_all(pool)
            .await
            .map_err(ServerFnError::new)?;

    let mut by_recipe: HashMap<i64, Vec<Ingredient>> = HashMap::new();
    for row in ingredient_rows {
        by_recipe
            .entry(row.get("recipe_id"))
            .or_default()
            .push(Ingredient {
                name: row.get("name"),
                quantity: row.get("quantity"),
                unit: row.get("unit"),
            });
    }

    for recipe in &mut recipes {
        if let Some(ingredients) = by_recipe.remove(&recipe.id) {
            recipe.ingredients = ingredients;
        }
    }

    Ok(recipes)
}

#[post("/api/recipes/create")]
pub async fn create_recipe(recipe: NewRecipe) -> Result<i64, ServerFnError> {
    let name = recipe.name.trim();
    if name.is_empty() {
        return Err(ServerFnError::new("A recipe needs a name"));
    }

    let pool = db().await.map_err(ServerFnError::new)?;

    // The recipe and its ingredients go in together, so a failure part way
    // through cannot leave a nameless recipe with no ingredients behind.
    let mut tx = pool.begin().await.map_err(ServerFnError::new)?;

    let id: i64 =
        sqlx::query("INSERT INTO recipes (name, instructions, servings) VALUES (?, ?, ?) RETURNING id")
            .bind(name)
            .bind(recipe.instructions.trim())
            .bind(recipe.servings.max(1))
            .fetch_one(&mut *tx)
            .await
            .map_err(ServerFnError::new)?
            .get("id");

    for ingredient in &recipe.ingredients {
        let ingredient_name = ingredient.name.trim();
        if ingredient_name.is_empty() {
            continue;
        }

        sqlx::query("INSERT INTO ingredients (recipe_id, name, quantity, unit) VALUES (?, ?, ?, ?)")
            .bind(id)
            .bind(ingredient_name)
            .bind(ingredient.quantity)
            .bind(ingredient.unit.trim())
            .execute(&mut *tx)
            .await
            .map_err(ServerFnError::new)?;
    }

    tx.commit().await.map_err(ServerFnError::new)?;

    Ok(id)
}

/// Attaches a photo to an existing recipe, replacing any earlier one.
///
/// Split from `create_recipe` rather than folded into `NewRecipe` so that
/// saving the text of a recipe never fails on account of the image, and so a
/// photo can be added to a recipe that was written down earlier.
#[post("/api/recipes/photo/set")]
pub async fn set_recipe_photo(
    id: i64,
    mime: String,
    data_base64: String,
) -> Result<(), ServerFnError> {
    let bytes = STANDARD
        .decode(data_base64.as_bytes())
        .map_err(|_| ServerFnError::new("That photo could not be decoded"))?;

    if bytes.is_empty() {
        return Err(ServerFnError::new("That photo was empty"));
    }

    if bytes.len() > MAX_PHOTO_BYTES {
        // One decimal place: whole megabytes round a 1.2 MB photo down to the
        // same "1 MB" as the limit it just exceeded.
        return Err(ServerFnError::new(format!(
            "That photo is {:.1} MB; the limit is {:.1} MB",
            bytes.len() as f64 / 1_048_576.0,
            MAX_PHOTO_BYTES as f64 / 1_048_576.0
        )));
    }

    // Anything but an image would still render as a broken picture rather than
    // execute, but there is no reason to keep bytes the app will never show.
    if !mime.starts_with("image/") {
        return Err(ServerFnError::new(format!("{mime} is not an image")));
    }

    let pool = db().await.map_err(ServerFnError::new)?;

    // Stamped on every write so the URL for this photo differs from the one
    // before it, which is what allows the response to be cached indefinitely.
    //
    // Taken as whichever is larger of the clock and one past the previous
    // version, so the value always moves even if two writes land in the same
    // millisecond. A repeated version would leave the browser holding an
    // `immutable` entry for a photo that had been replaced, which it would
    // never revalidate.
    let now_millis = chrono::Utc::now().timestamp_millis();

    let result = sqlx::query(
        "UPDATE recipes SET photo = ?, photo_mime = ?, \
         photo_version = MAX(COALESCE(photo_version, 0) + 1, ?) WHERE id = ?",
    )
    .bind(&bytes)
    .bind(&mime)
    .bind(now_millis)
    .bind(id)
    .execute(pool)
    .await
    .map_err(ServerFnError::new)?;

    if result.rows_affected() == 0 {
        return Err(ServerFnError::new("That recipe no longer exists"));
    }

    Ok(())
}

/// Drops a recipe's photo, leaving the recipe itself alone.
#[post("/api/recipes/photo/clear")]
pub async fn clear_recipe_photo(id: i64) -> Result<(), ServerFnError> {
    let pool = db().await.map_err(ServerFnError::new)?;

    sqlx::query("UPDATE recipes SET photo = NULL, photo_mime = NULL, photo_version = NULL WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(ServerFnError::new)?;

    Ok(())
}

#[post("/api/recipes/delete")]
pub async fn delete_recipe(id: i64) -> Result<(), ServerFnError> {
    let pool = db().await.map_err(ServerFnError::new)?;

    // Ingredients and any planned meals referencing this recipe are removed by
    // the ON DELETE CASCADE constraints.
    sqlx::query("DELETE FROM recipes WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(ServerFnError::new)?;

    Ok(())
}
