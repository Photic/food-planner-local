use crate::api::{create_recipe, delete_recipe, list_recipes};
use crate::models::{Ingredient, NewRecipe};
use dioxus::prelude::*;

#[component]
pub fn Recipes() -> Element {
    // Loaded on the client after hydration. Both server and client render the
    // same "Loading…" placeholder first, so there is no hydration mismatch.
    let mut recipes = use_resource(list_recipes);

    let list = match &*recipes.read() {
        None => rsx! { p { class: "muted", "Loading…" } },
        Some(Err(err)) => rsx! { p { class: "error", "Could not load recipes: {err}" } },
        Some(Ok(found)) if found.is_empty() => {
            rsx! { p { class: "muted", "No recipes yet. Add your first one below." } }
        }
        Some(Ok(found)) => rsx! {
            ul { class: "recipe-list",
                for recipe in found.iter() {
                    li { key: "{recipe.id}", class: "recipe",
                        div { class: "recipe-head",
                            h3 { "{recipe.name}" }
                            span { class: "muted", "serves {recipe.servings}" }
                            button {
                                class: "danger",
                                onclick: {
                                    let id = recipe.id;
                                    move |_| async move {
                                        if delete_recipe(id).await.is_ok() {
                                            recipes.restart();
                                        }
                                    }
                                },
                                "Delete"
                            }
                        }

                        if !recipe.ingredients.is_empty() {
                            ul { class: "ingredients",
                                for ingredient in recipe.ingredients.iter() {
                                    li { "{format_amount(ingredient)}{ingredient.name}" }
                                }
                            }
                        }

                        if !recipe.instructions.is_empty() {
                            p { class: "instructions", "{recipe.instructions}" }
                        }
                    }
                }
            }
        },
    };

    rsx! {
        section {
            h2 { "Recipes" }
            {list}
        }

        RecipeForm { on_saved: move |_| recipes.restart() }
    }
}

/// Renders the quantity and unit prefix, omitting whatever was left blank so a
/// bare "Salt" does not come out as "0  Salt".
fn format_amount(ingredient: &Ingredient) -> String {
    let quantity = if ingredient.quantity == 0.0 {
        String::new()
    } else {
        // Trim the trailing ".0" that floats print for whole numbers.
        let rendered = format!("{}", ingredient.quantity);
        rendered
    };

    match (quantity.is_empty(), ingredient.unit.is_empty()) {
        (true, true) => String::new(),
        (true, false) => format!("{} ", ingredient.unit),
        (false, true) => format!("{} ", quantity),
        (false, false) => format!("{} {} ", quantity, ingredient.unit),
    }
}

#[component]
fn RecipeForm(on_saved: EventHandler<()>) -> Element {
    let mut name = use_signal(String::new);
    let mut servings = use_signal(|| 2_i64);
    let mut instructions = use_signal(String::new);
    let mut ingredients = use_signal(|| vec![Ingredient::default()]);
    let mut error = use_signal(|| Option::<String>::None);
    let mut saving = use_signal(|| false);

    let submit = move |_| async move {
        saving.set(true);
        error.set(None);

        let recipe = NewRecipe {
            name: name(),
            instructions: instructions(),
            servings: servings(),
            ingredients: ingredients(),
        };

        match create_recipe(recipe).await {
            Ok(_) => {
                name.set(String::new());
                instructions.set(String::new());
                servings.set(2);
                ingredients.set(vec![Ingredient::default()]);
                on_saved.call(());
            }
            Err(err) => error.set(Some(err.to_string())),
        }

        saving.set(false);
    };

    rsx! {
        section { class: "card",
            h2 { "Add a recipe" }

            label { "Name"
                input {
                    value: "{name}",
                    placeholder: "Spaghetti carbonara",
                    oninput: move |event| name.set(event.value()),
                }
            }

            label { "Servings"
                input {
                    r#type: "number",
                    min: "1",
                    value: "{servings}",
                    oninput: move |event| {
                        if let Ok(parsed) = event.value().parse::<i64>() {
                            servings.set(parsed.max(1));
                        }
                    },
                }
            }

            h3 { "Ingredients" }
            for (index, ingredient) in ingredients().into_iter().enumerate() {
                div { key: "{index}", class: "ingredient-row",
                    input {
                        class: "qty",
                        r#type: "number",
                        step: "any",
                        placeholder: "0",
                        value: if ingredient.quantity == 0.0 { String::new() } else { ingredient.quantity.to_string() },
                        oninput: move |event| {
                            let parsed = event.value().parse::<f64>().unwrap_or(0.0);
                            ingredients.write()[index].quantity = parsed;
                        },
                    }
                    input {
                        class: "unit",
                        placeholder: "g",
                        value: "{ingredient.unit}",
                        oninput: move |event| ingredients.write()[index].unit = event.value(),
                    }
                    input {
                        class: "ingredient-name",
                        placeholder: "pancetta",
                        value: "{ingredient.name}",
                        oninput: move |event| ingredients.write()[index].name = event.value(),
                    }
                    button {
                        class: "danger",
                        // Keep one row on screen so the form never becomes a dead end.
                        disabled: ingredients.read().len() <= 1,
                        onclick: move |_| { ingredients.write().remove(index); },
                        "×"
                    }
                }
            }

            button {
                onclick: move |_| ingredients.write().push(Ingredient::default()),
                "Add ingredient"
            }

            label { "Instructions"
                textarea {
                    rows: "4",
                    placeholder: "Boil the pasta…",
                    value: "{instructions}",
                    oninput: move |event| instructions.set(event.value()),
                }
            }

            if let Some(message) = error() {
                p { class: "error", "{message}" }
            }

            button {
                class: "primary",
                disabled: saving(),
                onclick: submit,
                if saving() { "Saving…" } else { "Save recipe" }
            }
        }
    }
}
