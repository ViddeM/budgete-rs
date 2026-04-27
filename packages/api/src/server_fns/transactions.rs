use crate::models::{CsvSource, ImportResult, QueueState, Transaction, TransactionFilter};
use dioxus::prelude::*;

#[cfg(feature = "server")]
use {
    crate::auth::session::current_user_id,
    crate::csv,
    crate::db::pool,
    crate::db_rows::TransactionRow,
    sha2::{Digest, Sha256},
    std::fmt::Write as _,
};

/// Import a CSV file for the current user. Returns counts of imported /
/// skipped / pending rows.
#[server]
pub async fn import_csv(source: CsvSource, content: String) -> Result<ImportResult, ServerFnError> {
    let user_id = current_user_id().await?;

    let rows = match source {
        CsvSource::Amex => csv::amex::parse(&content),
        CsvSource::Nordea => csv::nordea::parse(&content),
    }
    .map_err(ServerFnError::new)?;

    let db = pool();
    let source_str = source.to_string();
    let mut imported: u32 = 0;
    let mut skipped: u32 = 0;
    let mut pending: u32 = 0;

    for row in rows {
        let date_str = row
            .date
            .map(|d| d.to_string())
            .unwrap_or_else(|| "pending".to_string());
        let hash_input = format!(
            "{}|{}|{}|{}",
            source_str, date_str, row.description, row.amount
        );
        let mut hasher = Sha256::new();
        hasher.update(hash_input.as_bytes());
        let hash_bytes = hasher.finalize();
        let mut dedup_hash = String::with_capacity(64);
        for b in hash_bytes {
            write!(dedup_hash, "{b:02x}").unwrap();
        }

        let result = sqlx::query(
            r#"
            INSERT INTO transactions (date, description, amount, source, currency, dedup_hash, is_pending, user_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (user_id, dedup_hash) DO NOTHING
            "#,
        )
        .bind(row.date)
        .bind(&row.description)
        .bind(row.amount)
        .bind(&source_str)
        .bind(&row.currency)
        .bind(&dedup_hash)
        .bind(row.is_pending)
        .bind(user_id)
        .execute(db)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        if result.rows_affected() == 0 {
            skipped += 1;
        } else if row.is_pending {
            pending += 1;
        } else {
            imported += 1;
        }
    }

    Ok(ImportResult { imported, skipped, pending })
}

/// Fetch transactions for the current user with optional filtering.
#[server]
pub async fn get_transactions(
    filter: TransactionFilter,
) -> Result<Vec<Transaction>, ServerFnError> {
    let user_id = current_user_id().await?;
    let db = pool();

    let rows: Vec<TransactionRow> = sqlx::query_as(
        r#"
        SELECT
            t.id,
            t.date,
            t.description,
            t.amount,
            t.source,
            t.currency,
            t.is_pending,
            t.category_id,
            c.name     AS category_name,
            c.color    AS category_color,
            c.parent_id AS category_parent_id
        FROM transactions t
        LEFT JOIN categories c ON c.id = t.category_id
        WHERE t.user_id = $1
            AND ($2::boolean = false OR t.category_id IS NULL)
            AND ($3::uuid IS NULL OR t.category_id = $3)
            AND ($4::uuid IS NULL OR EXISTS (
                SELECT 1 FROM transaction_groups tg
                WHERE tg.transaction_id = t.id AND tg.group_id = $4
            ))
            AND ($5::date IS NULL OR t.date >= $5)
            AND ($6::date IS NULL OR t.date <= $6)
        ORDER BY t.date DESC NULLS LAST, t.created_at DESC
        "#,
    )
    .bind(user_id)
    .bind(filter.unprocessed_only)
    .bind(filter.category_id)
    .bind(filter.group_id)
    .bind(filter.date_from)
    .bind(filter.date_to)
    .fetch_all(db)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(rows.into_iter().map(Into::into).collect())
}

/// Return the next unclassified (non-pending) transaction for the current user
/// and the total remaining count. The queue is ordered oldest-first.
#[server]
pub async fn get_queue_state() -> Result<QueueState, ServerFnError> {
    let user_id = current_user_id().await?;
    let db = pool();

    let remaining: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM transactions \
         WHERE user_id = $1 AND category_id IS NULL AND is_pending = false",
    )
    .bind(user_id)
    .fetch_one(db)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    let rows: Vec<TransactionRow> = sqlx::query_as(
        r#"
        SELECT
            t.id,
            t.date,
            t.description,
            t.amount,
            t.source,
            t.currency,
            t.is_pending,
            t.category_id,
            c.name     AS category_name,
            c.color    AS category_color,
            c.parent_id AS category_parent_id
        FROM transactions t
        LEFT JOIN categories c ON c.id = t.category_id
        WHERE t.user_id = $1 AND t.category_id IS NULL AND t.is_pending = false
        ORDER BY t.date ASC NULLS LAST, t.created_at ASC
        LIMIT 4
        "#,
    )
    .bind(user_id)
    .fetch_all(db)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    let mut items: Vec<Transaction> = rows.into_iter().map(Into::into).collect();
    let next = if items.is_empty() { None } else { Some(items.remove(0)) };
    let upcoming = items; // at most 3

    Ok(QueueState { next, upcoming, remaining })
}
