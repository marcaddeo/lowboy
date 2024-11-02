use anyhow::Result;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;

// This builds out a database from migrations that can be used by SQLx to verify queries at
// compile-time.
async fn scaffold_build_db() -> Result<SqlitePool> {
    use std::path::Path;

    let crate_dir = std::env::var("CARGO_MANIFEST_DIR")?;
    let database = Path::new(&crate_dir).join("./target/database.sqlite3");
    let migrations = Path::new(&crate_dir).join("./migrations");

    let options = SqliteConnectOptions::new()
        .filename(database)
        .create_if_missing(true);

    let db = SqlitePool::connect_with(options).await?;

    sqlx::migrate::Migrator::new(migrations)
        .await?
        .run(&db)
        .await?;

    Ok(db)
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=migrations");

    scaffold_build_db().await?;

    Ok(())
}
