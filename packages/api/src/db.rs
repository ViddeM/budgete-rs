#[cfg(feature = "server")]
use sqlx::PgPool;

#[cfg(feature = "server")]
static POOL: std::sync::OnceLock<PgPool> = std::sync::OnceLock::new();

/// Initialize the database pool from the validated config and run migrations.
///
/// The connection URL is taken from [`crate::config::config()`], so
/// [`crate::config::load()`] must be called before this function.
///
/// Safe to call multiple times — only the first call connects and migrates.
#[cfg(feature = "server")]
pub async fn init_pool() -> Result<(), sqlx::Error> {
    if POOL.get().is_some() {
        return Ok(());
    }
    let url = &crate::config::config().database_url;
    let pool = PgPool::connect(url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    // Ignore the error if another task raced us to set the pool.
    let _ = POOL.set(pool);
    Ok(())
}

/// Get a reference to the global pool. Panics if `init_pool` has not been called.
#[cfg(feature = "server")]
pub fn pool() -> &'static PgPool {
    POOL.get().expect("database pool not initialized — call init_pool() at startup")
}
