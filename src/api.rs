//! Server functions: the boundary between the browser and SQLite.
//!
//! Each one compiles twice. On the server the body runs against the database;
//! on the client the macro replaces it with an HTTP call to the same path.

use crate::models::*;
use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::db::db;
#[cfg(feature = "server")]
use sqlx::Row;

#[get("/api/recipes")]
pub async fn list_recipes() -> Result<Vec<Recipe>, ServerFnError> {
    use std::collections::HashMap;

    let pool = db().await.map_err(ServerFnError::new)?;

    let recipe_rows =
        sqlx::query("SELECT id, name, instructions, servings FROM recipes ORDER BY name COLLATE NOCASE")
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
