//! SQLite connection handling. Server-side only.

use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::str::FromStr;
use tokio::sync::OnceCell;

static POOL: OnceCell<SqlitePool> = OnceCell::const_new();

/// Where the database file lives. Override with `FOOD_PLANNER_DB` to keep the
/// data somewhere other than the working directory.
fn database_path() -> String {
    std::env::var("FOOD_PLANNER_DB").unwrap_or_else(|_| "food-planner.db".to_string())
}

/// Returns the shared pool, opening the database and running migrations the
/// first time it is called.
pub async fn db() -> Result<&'static SqlitePool, sqlx::Error> {
    POOL.get_or_try_init(|| async {
        let options = SqliteConnectOptions::from_str(&database_path())?
            .create_if_missing(true)
            // SQLite leaves foreign keys unenforced unless asked, which would
            // let ingredients outlive the recipe they belong to.
            .foreign_keys(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(pool)
    })
    .await
}
