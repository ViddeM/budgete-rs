//! Integration tests for the database layer.
//!
//! Each test runs in an isolated database created and migrated by `sqlx::test`,
//! then dropped automatically on teardown.
//!
//! These tests are gated behind the `integration-tests` feature so that a plain
//! `cargo test` (which has no Postgres available) never runs them. To run them,
//! set `DATABASE_URL` or `TEST_DATABASE_URL` pointing to a Postgres instance and
//! enable the feature:
//!
//!   TEST_DATABASE_URL=postgres://user:pass@localhost/postgres \
//!     cargo test -p api --features integration-tests --test db_integration

#[cfg(feature = "integration-tests")]
use chrono::NaiveDate;
#[cfg(feature = "integration-tests")]
use rust_decimal::Decimal;
#[cfg(feature = "integration-tests")]
use sqlx::PgPool;
#[cfg(feature = "integration-tests")]
use uuid::Uuid;

// ── helpers ──────────────────────────────────────────────────────────────────

#[cfg(feature = "integration-tests")]
async fn create_household(pool: &PgPool) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO households (name, invite_code) VALUES ('Test Household', $1) RETURNING id",
    )
    .bind(Uuid::new_v4().to_string())
    .fetch_one(pool)
    .await
    .unwrap()
}

#[cfg(feature = "integration-tests")]
async fn insert_tx(
    pool: &PgPool,
    household_id: Uuid,
    date: NaiveDate,
    description: &str,
    amount: Decimal,
    dedup_hash: &str,
) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO transactions \
         (date, description, amount, source, currency, dedup_hash, is_pending, household_id) \
         VALUES ($1, $2, $3, 'amex', 'SEK', $4, false, $5) \
         RETURNING id",
    )
    .bind(date)
    .bind(description)
    .bind(amount)
    .bind(dedup_hash)
    .bind(household_id)
    .fetch_one(pool)
    .await
    .unwrap()
}

#[cfg(feature = "integration-tests")]
async fn insert_category(pool: &PgPool, household_id: Uuid, name: &str, color: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO categories (name, color, household_id, ignored) \
         VALUES ($1, $2, $3, false) RETURNING id",
    )
    .bind(name)
    .bind(color)
    .bind(household_id)
    .fetch_one(pool)
    .await
    .unwrap()
}

#[cfg(feature = "integration-tests")]
async fn insert_subcategory(
    pool: &PgPool,
    household_id: Uuid,
    parent_id: Uuid,
    name: &str,
    color: &str,
) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO categories (name, color, parent_id, household_id, ignored) \
         VALUES ($1, $2, $3, $4, false) RETURNING id",
    )
    .bind(name)
    .bind(color)
    .bind(parent_id)
    .bind(household_id)
    .fetch_one(pool)
    .await
    .unwrap()
}

#[cfg(feature = "integration-tests")]
fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

#[cfg(feature = "integration-tests")]
fn dec(s: &str) -> Decimal {
    s.parse().unwrap()
}

// ── deduplication ────────────────────────────────────────────────────────────

#[cfg(feature = "integration-tests")]
#[sqlx::test]
async fn on_conflict_skips_duplicate_in_same_household(pool: PgPool) {
    let hid = create_household(&pool).await;

    insert_tx(
        &pool,
        hid,
        date(2026, 1, 15),
        "ICA FOCUS",
        dec("-100.00"),
        "hash-001",
    )
    .await;

    // Second insert with same household + dedup_hash: ON CONFLICT DO NOTHING.
    let affected = sqlx::query(
        "INSERT INTO transactions \
         (date, description, amount, source, currency, dedup_hash, is_pending, household_id) \
         VALUES ('2026-01-15', 'ICA FOCUS', -100.00, 'amex', 'SEK', $1, false, $2) \
         ON CONFLICT (household_id, dedup_hash) DO NOTHING",
    )
    .bind("hash-001")
    .bind(hid)
    .execute(&pool)
    .await
    .unwrap()
    .rows_affected();

    assert_eq!(affected, 0, "duplicate within household should be skipped");

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM transactions WHERE household_id = $1")
            .bind(hid)
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(count, 1);
}

#[cfg(feature = "integration-tests")]
#[sqlx::test]
async fn same_dedup_hash_allowed_across_households(pool: PgPool) {
    let h1 = create_household(&pool).await;
    let h2 = create_household(&pool).await;

    insert_tx(
        &pool,
        h1,
        date(2026, 1, 15),
        "ICA",
        dec("-100.00"),
        "shared-hash",
    )
    .await;
    insert_tx(
        &pool,
        h2,
        date(2026, 1, 15),
        "ICA",
        dec("-100.00"),
        "shared-hash",
    )
    .await;

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM transactions")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(
        total, 2,
        "same hash in different households should both be stored"
    );
}

// ── household isolation ───────────────────────────────────────────────────────

#[cfg(feature = "integration-tests")]
#[sqlx::test]
async fn transactions_isolated_by_household(pool: PgPool) {
    let h1 = create_household(&pool).await;
    let h2 = create_household(&pool).await;

    insert_tx(
        &pool,
        h1,
        date(2026, 1, 15),
        "Expense A",
        dec("-100.00"),
        "hash-h1",
    )
    .await;

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM transactions WHERE household_id = $1")
            .bind(h2)
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(
        count, 0,
        "household B should not see household A's transactions"
    );
}

// ── category hierarchy ───────────────────────────────────────────────────────

#[cfg(feature = "integration-tests")]
#[sqlx::test]
async fn category_list_query_orders_parents_before_subcategories(pool: PgPool) {
    let hid = create_household(&pool).await;

    let food_id = insert_category(&pool, hid, "Food", "#ff0000").await;
    let transport_id = insert_category(&pool, hid, "Transport", "#00ff00").await;
    let groceries_id = insert_subcategory(&pool, hid, food_id, "Groceries", "#ff8800").await;
    let restaurant_id = insert_subcategory(&pool, hid, food_id, "Restaurant", "#ff4400").await;
    let _ = transport_id;

    let rows: Vec<(Uuid, Option<Uuid>)> = sqlx::query_as(
        "SELECT id, parent_id FROM categories WHERE household_id = $1 \
         ORDER BY COALESCE(parent_id, id), parent_id NULLS FIRST, name",
    )
    .bind(hid)
    .fetch_all(&pool)
    .await
    .unwrap();

    let pos = |target: Uuid| rows.iter().position(|(id, _)| *id == target).unwrap();

    assert!(
        pos(food_id) < pos(groceries_id),
        "parent Food should appear before subcategory Groceries"
    );
    assert!(
        pos(food_id) < pos(restaurant_id),
        "parent Food should appear before subcategory Restaurant"
    );
}

#[cfg(feature = "integration-tests")]
#[sqlx::test]
async fn delete_parent_cascades_and_unclassifies_transactions(pool: PgPool) {
    let hid = create_household(&pool).await;

    let parent_id = insert_category(&pool, hid, "Food", "#ff0000").await;
    let sub_id = insert_subcategory(&pool, hid, parent_id, "Groceries", "#00ff00").await;

    let tx_id = insert_tx(
        &pool,
        hid,
        date(2026, 1, 15),
        "ICA",
        dec("-100.00"),
        "hash-casc",
    )
    .await;
    sqlx::query("UPDATE transactions SET category_id = $1 WHERE id = $2")
        .bind(sub_id)
        .bind(tx_id)
        .execute(&pool)
        .await
        .unwrap();

    // Deleting the parent category cascades to the subcategory (ON DELETE CASCADE),
    // which then triggers ON DELETE SET NULL on the transaction.
    sqlx::query("DELETE FROM categories WHERE id = $1")
        .bind(parent_id)
        .execute(&pool)
        .await
        .unwrap();

    let sub_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM categories WHERE id = $1")
        .bind(sub_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(sub_count, 0, "subcategory should be cascade-deleted");

    let cat_id: Option<Uuid> =
        sqlx::query_scalar("SELECT category_id FROM transactions WHERE id = $1")
            .bind(tx_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        cat_id.is_none(),
        "transaction should be unclassified after its category is deleted"
    );
}

// ── transaction filters ───────────────────────────────────────────────────────

#[cfg(feature = "integration-tests")]
#[sqlx::test]
async fn unprocessed_filter_excludes_classified_and_pending(pool: PgPool) {
    let hid = create_household(&pool).await;
    let cat_id = insert_category(&pool, hid, "Food", "#ff0000").await;

    let classified_id = insert_tx(
        &pool,
        hid,
        date(2026, 1, 15),
        "Classified",
        dec("-100.00"),
        "hash-cl",
    )
    .await;
    let _unclassified_id = insert_tx(
        &pool,
        hid,
        date(2026, 1, 14),
        "Unclassified",
        dec("-50.00"),
        "hash-un",
    )
    .await;

    // Insert a pending transaction (is_pending = true).
    sqlx::query(
        "INSERT INTO transactions \
         (date, description, amount, source, currency, dedup_hash, is_pending, household_id) \
         VALUES ($1, 'Pending purchase', -75.00, 'nordea', 'SEK', 'hash-pend', true, $2)",
    )
    .bind(date(2026, 1, 13))
    .bind(hid)
    .execute(&pool)
    .await
    .unwrap();

    // Classify the first transaction.
    sqlx::query("UPDATE transactions SET category_id = $1 WHERE id = $2")
        .bind(cat_id)
        .bind(classified_id)
        .execute(&pool)
        .await
        .unwrap();

    // Unprocessed = category IS NULL AND NOT pending.
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM transactions \
         WHERE household_id = $1 AND category_id IS NULL AND is_pending = false",
    )
    .bind(hid)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(
        count, 1,
        "only one unclassified, non-pending transaction expected"
    );
}

#[cfg(feature = "integration-tests")]
#[sqlx::test]
async fn date_range_filter_returns_matching_transactions(pool: PgPool) {
    let hid = create_household(&pool).await;

    insert_tx(
        &pool,
        hid,
        date(2026, 1, 10),
        "January",
        dec("-100.00"),
        "hash-jan",
    )
    .await;
    insert_tx(
        &pool,
        hid,
        date(2026, 2, 15),
        "February",
        dec("-200.00"),
        "hash-feb",
    )
    .await;
    insert_tx(
        &pool,
        hid,
        date(2026, 3, 20),
        "March",
        dec("-300.00"),
        "hash-mar",
    )
    .await;

    let descriptions: Vec<String> = sqlx::query_scalar(
        "SELECT description FROM transactions \
         WHERE household_id = $1 AND date >= $2 AND date <= $3 \
         ORDER BY date",
    )
    .bind(hid)
    .bind(date(2026, 2, 1))
    .bind(date(2026, 2, 28))
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(descriptions, vec!["February"]);
}

#[cfg(feature = "integration-tests")]
#[sqlx::test]
async fn category_filter_returns_only_matching_transactions(pool: PgPool) {
    let hid = create_household(&pool).await;
    let food_id = insert_category(&pool, hid, "Food", "#ff0000").await;
    let transport_id = insert_category(&pool, hid, "Transport", "#00ff00").await;

    let food_tx = insert_tx(
        &pool,
        hid,
        date(2026, 1, 10),
        "ICA",
        dec("-100.00"),
        "hash-food",
    )
    .await;
    let transport_tx = insert_tx(
        &pool,
        hid,
        date(2026, 1, 11),
        "SL Card",
        dec("-50.00"),
        "hash-sl",
    )
    .await;

    sqlx::query("UPDATE transactions SET category_id = $1 WHERE id = $2")
        .bind(food_id)
        .bind(food_tx)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE transactions SET category_id = $1 WHERE id = $2")
        .bind(transport_id)
        .bind(transport_tx)
        .execute(&pool)
        .await
        .unwrap();

    let descriptions: Vec<String> = sqlx::query_scalar(
        "SELECT description FROM transactions \
         WHERE household_id = $1 AND category_id = $2",
    )
    .bind(hid)
    .bind(food_id)
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(descriptions, vec!["ICA"]);
}

// ── dashboard: expenses / income aggregation ─────────────────────────────────

#[cfg(feature = "integration-tests")]
#[sqlx::test]
async fn dashboard_aggregation_sums_expenses_and_income(pool: PgPool) {
    let hid = create_household(&pool).await;

    insert_tx(
        &pool,
        hid,
        date(2026, 6, 1),
        "Expense A",
        dec("-100.00"),
        "hash-e1",
    )
    .await;
    insert_tx(
        &pool,
        hid,
        date(2026, 6, 5),
        "Expense B",
        dec("-250.00"),
        "hash-e2",
    )
    .await;
    insert_tx(
        &pool,
        hid,
        date(2026, 6, 10),
        "Income",
        dec("3000.00"),
        "hash-i1",
    )
    .await;

    let month_start = date(2026, 6, 1);
    let (expenses, income): (Decimal, Decimal) = sqlx::query_as(
        "SELECT \
           COALESCE(SUM(amount) FILTER (WHERE amount < 0), 0), \
           COALESCE(SUM(amount) FILTER (WHERE amount > 0), 0) \
         FROM transactions t \
         LEFT JOIN categories c ON c.id = t.category_id \
         WHERE t.household_id = $1 AND t.date >= $2 AND t.is_pending = false \
           AND COALESCE(c.ignored, false) = false",
    )
    .bind(hid)
    .bind(month_start)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(expenses, dec("-350.00"));
    assert_eq!(income, dec("3000.00"));
}

#[cfg(feature = "integration-tests")]
#[sqlx::test]
async fn dashboard_excludes_ignored_categories(pool: PgPool) {
    let hid = create_household(&pool).await;

    // Insert an "ignored" category.
    let ignored_id: Uuid = sqlx::query_scalar(
        "INSERT INTO categories (name, color, household_id, ignored) \
         VALUES ('Savings transfer', '#888888', $1, true) RETURNING id",
    )
    .bind(hid)
    .fetch_one(&pool)
    .await
    .unwrap();

    let normal_tx = insert_tx(
        &pool,
        hid,
        date(2026, 6, 1),
        "Coffee",
        dec("-50.00"),
        "hash-cof",
    )
    .await;
    let ignored_tx = insert_tx(
        &pool,
        hid,
        date(2026, 6, 2),
        "Savings",
        dec("-1000.00"),
        "hash-sav",
    )
    .await;

    sqlx::query("UPDATE transactions SET category_id = $1 WHERE id = $2")
        .bind(ignored_id)
        .bind(ignored_tx)
        .execute(&pool)
        .await
        .unwrap();

    let _ = normal_tx;

    let expenses: Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount) FILTER (WHERE amount < 0), 0) \
         FROM transactions t \
         LEFT JOIN categories c ON c.id = t.category_id \
         WHERE t.household_id = $1 AND t.date >= $2 AND t.is_pending = false \
           AND COALESCE(c.ignored, false) = false",
    )
    .bind(hid)
    .bind(date(2026, 6, 1))
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(
        expenses,
        dec("-50.00"),
        "ignored category transactions should be excluded from totals"
    );
}

// ── spending over time ────────────────────────────────────────────────────────

#[cfg(feature = "integration-tests")]
#[sqlx::test]
async fn spending_over_time_groups_by_calendar_month(pool: PgPool) {
    let hid = create_household(&pool).await;

    insert_tx(
        &pool,
        hid,
        date(2026, 1, 10),
        "Jan A",
        dec("-100.00"),
        "hash-j1",
    )
    .await;
    insert_tx(
        &pool,
        hid,
        date(2026, 1, 20),
        "Jan B",
        dec("-200.00"),
        "hash-j2",
    )
    .await;
    insert_tx(
        &pool,
        hid,
        date(2026, 2, 5),
        "Feb A",
        dec("-150.00"),
        "hash-f1",
    )
    .await;
    insert_tx(
        &pool,
        hid,
        date(2026, 2, 15),
        "Feb salary",
        dec("3000.00"),
        "hash-f2",
    )
    .await;

    let rows: Vec<(String, Decimal, Decimal)> = sqlx::query_as(
        "SELECT \
           TO_CHAR(DATE_TRUNC('month', t.date), 'YYYY-MM') AS period_label, \
           SUM(CASE WHEN t.amount < 0 THEN -t.amount ELSE 0::numeric END) AS expenses, \
           SUM(CASE WHEN t.amount > 0 THEN  t.amount ELSE 0::numeric END) AS income \
         FROM transactions t \
         LEFT JOIN categories c ON c.id = t.category_id \
         WHERE t.household_id = $1 AND t.date >= $2 AND t.date <= $3 \
           AND t.is_pending = false \
           AND COALESCE(c.ignored, false) = false \
         GROUP BY DATE_TRUNC('month', t.date) \
         ORDER BY DATE_TRUNC('month', t.date)",
    )
    .bind(hid)
    .bind(date(2026, 1, 1))
    .bind(date(2026, 12, 31))
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].0, "2026-01");
    assert_eq!(rows[0].1, dec("300.00"), "January expenses");
    assert_eq!(rows[0].2, dec("0"), "January income");
    assert_eq!(rows[1].0, "2026-02");
    assert_eq!(rows[1].1, dec("150.00"), "February expenses");
    assert_eq!(rows[1].2, dec("3000.00"), "February income");
}
