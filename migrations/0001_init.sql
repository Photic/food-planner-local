-- Recipes are the foundation: the meal plan references them, and the shopping
-- list is derived by rolling up the ingredients of everything planned.
CREATE TABLE recipes (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    name         TEXT    NOT NULL,
    instructions TEXT    NOT NULL DEFAULT '',
    servings     INTEGER NOT NULL DEFAULT 2
);

CREATE TABLE ingredients (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    recipe_id INTEGER NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    name      TEXT    NOT NULL,
    quantity  REAL    NOT NULL DEFAULT 0,
    -- Free text ("g", "tbsp", "cloves"). Quantities only aggregate across
    -- entries that share both a name and a unit.
    unit      TEXT    NOT NULL DEFAULT ''
);

CREATE INDEX idx_ingredients_recipe ON ingredients(recipe_id);

-- One recipe per (day, meal slot). Re-planning a slot replaces what was there.
CREATE TABLE meal_plan (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    date      TEXT    NOT NULL, -- ISO-8601 YYYY-MM-DD
    slot      TEXT    NOT NULL, -- breakfast | lunch | dinner
    recipe_id INTEGER NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    UNIQUE (date, slot)
);

CREATE INDEX idx_meal_plan_date ON meal_plan(date);

-- What is already at home, subtracted from the generated shopping list.
CREATE TABLE pantry (
    id       INTEGER PRIMARY KEY AUTOINCREMENT,
    name     TEXT    NOT NULL,
    quantity REAL    NOT NULL DEFAULT 0,
    unit     TEXT    NOT NULL DEFAULT '',
    UNIQUE (name, unit)
);

-- The shopping list itself is computed on demand from meal_plan + pantry, so
-- only the tick-box state needs storing. Keyed by "lowercased name|unit".
CREATE TABLE shopping_list_state (
    item_key TEXT    PRIMARY KEY,
    checked  INTEGER NOT NULL DEFAULT 0
);
