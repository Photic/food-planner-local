//! Types shared between the client and the server.
//!
//! Everything here crosses the network as JSON in server function arguments and
//! return values, so it must compile for both the wasm and native targets.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recipe {
    pub id: i64,
    pub name: String,
    pub instructions: String,
    pub servings: i64,
    pub ingredients: Vec<Ingredient>,
    /// Identifies the current photo, and is `None` when there is none. Not the
    /// photo itself: the listing returns every recipe at once, so carrying
    /// image bytes here would put the whole album in one response. The bytes
    /// come from `/photo/{id}`, and this goes in its query string so that
    /// replacing a photo produces a URL the browser has not cached.
    pub photo_version: Option<i64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Ingredient {
    pub name: String,
    pub quantity: f64,
    pub unit: String,
}

/// A recipe submitted from the browser, before the database assigns an id.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NewRecipe {
    pub name: String,
    pub instructions: String,
    pub servings: i64,
    pub ingredients: Vec<Ingredient>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MealSlot {
    Breakfast,
    Lunch,
    Dinner,
}

impl MealSlot {
    pub const ALL: [MealSlot; 3] = [MealSlot::Breakfast, MealSlot::Lunch, MealSlot::Dinner];

    pub fn as_str(self) -> &'static str {
        match self {
            MealSlot::Breakfast => "breakfast",
            MealSlot::Lunch => "lunch",
            MealSlot::Dinner => "dinner",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            MealSlot::Breakfast => "Breakfast",
            MealSlot::Lunch => "Lunch",
            MealSlot::Dinner => "Dinner",
        }
    }
}

/// One planned meal: a recipe assigned to a date and slot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannedMeal {
    pub date: String, // ISO-8601 YYYY-MM-DD
    pub slot: MealSlot,
    pub recipe_id: i64,
    pub recipe_name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PantryItem {
    pub id: i64,
    pub name: String,
    pub quantity: f64,
    pub unit: String,
}

/// A line on the generated shopping list, after pantry stock is subtracted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShoppingItem {
    /// Stable identity for the tick-box state: "lowercased name|unit".
    pub key: String,
    pub name: String,
    pub quantity: f64,
    pub unit: String,
    pub checked: bool,
}

/// Groups quantities that can be summed. Two ingredients only combine when they
/// agree on both name and unit, since "2 cloves garlic" and "10 g garlic" have
/// no common scale.
pub fn item_key(name: &str, unit: &str) -> String {
    format!("{}|{}", name.trim().to_lowercase(), unit.trim().to_lowercase())
}
