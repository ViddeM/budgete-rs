#[cfg(feature = "server")]
use sqlx::PgPool;

#[cfg(feature = "server")]
static POOL: std::sync::OnceLock<PgPool> = std::sync::OnceLock::new();

/// Initialize the database pool from the DATABASE_URL environment variable.
/// Runs migrations automatically on startup.
#[cfg(feature = "server")]
pub async fn init_pool() -> Result<(), sqlx::Error> {
    dotenvy::dotenv().ok();
    let url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    POOL.set(pool).expect("pool already initialized");
    Ok(())
}

/// Get a reference to the global pool. Panics if `init_pool` has not been called.
#[cfg(feature = "server")]
pub fn pool() -> &'static PgPool {
    POOL.get().expect("database pool not initialized — call init_pool() at startup")
}
