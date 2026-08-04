use crate::models::{ImportResult, ImportSource, QueueState, Transaction, TransactionFilter};
use dioxus::prelude::*;

#[cfg(feature = "server")]
use {
    crate::auth::session::current_household_id,
    crate::csv,
    crate::db::pool,
    crate::db_rows::TransactionRow,
    base64::Engine as _,
    sha2::{Digest, Sha256},
    std::fmt::Write as _,
};

/// Compute the dedup hash for raw transaction fields.
/// This is the single source of truth used by CSV imports and manual entry.
#[cfg(feature = "server")]
fn compute_dedup_hash(
    source_str: &str,
    date: Option<chrono::NaiveDate>,
    description: &str,
    amount: rust_decimal::Decimal,
) -> String {
    let date_str = date
        .map(|d| d.to_string())
        .unwrap_or_else(|| "pending".to_string());
    let hash_input = format!("{}|{}|{}|{}", source_str, date_str, description, amount);
    let mut hasher = Sha256::new();
    hasher.update(hash_input.as_bytes());
    let hash_bytes = hasher.finalize();
    let mut dedup_hash = String::with_capacity(64);
    for b in hash_bytes {
        write!(dedup_hash, "{b:02x}").unwrap();
    }
    dedup_hash
}

/// Compute the dedup hash for a parsed CSV row.
#[cfg(feature = "server")]
fn compute_dedup_hash_for_row(source_str: &str, row: &crate::csv::ParsedRow) -> String {
    compute_dedup_hash(source_str, row.date, &row.description, row.amount)
}

/// Parse the import content for any supported source into [`ParsedRow`]s.
#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;
    use crate::csv::ParsedRow;
    use chrono::NaiveDate;

    fn dated_row(date: NaiveDate, description: &str, amount: &str) -> ParsedRow {
        ParsedRow {
            date: Some(date),
            description: description.to_string(),
            amount: amount.parse().unwrap(),
            currency: "SEK".to_string(),
            is_pending: false,
        }
    }

    #[test]
    fn hash_is_deterministic() {
        let row = dated_row(
            NaiveDate::from_ymd_opt(2026, 1, 15).unwrap(),
            "ICA FOCUS",
            "-100.50",
        );
        assert_eq!(
            compute_dedup_hash_for_row("amex", &row),
            compute_dedup_hash_for_row("amex", &row)
        );
    }

    #[test]
    fn hash_is_64_hex_chars() {
        let row = dated_row(
            NaiveDate::from_ymd_opt(2026, 1, 15).unwrap(),
            "ICA FOCUS",
            "-100.00",
        );
        let hash = compute_dedup_hash_for_row("amex", &row);
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn hash_differs_for_different_sources() {
        let row = dated_row(
            NaiveDate::from_ymd_opt(2026, 1, 15).unwrap(),
            "ICA FOCUS",
            "-100.00",
        );
        assert_ne!(
            compute_dedup_hash_for_row("amex", &row),
            compute_dedup_hash_for_row("nordea", &row)
        );
    }

    #[test]
    fn hash_differs_for_different_amounts() {
        let r1 = dated_row(
            NaiveDate::from_ymd_opt(2026, 1, 15).unwrap(),
            "ICA FOCUS",
            "-100.00",
        );
        let r2 = dated_row(
            NaiveDate::from_ymd_opt(2026, 1, 15).unwrap(),
            "ICA FOCUS",
            "-200.00",
        );
        assert_ne!(
            compute_dedup_hash_for_row("amex", &r1),
            compute_dedup_hash_for_row("amex", &r2)
        );
    }

    #[test]
    fn hash_differs_for_different_descriptions() {
        let r1 = dated_row(
            NaiveDate::from_ymd_opt(2026, 1, 15).unwrap(),
            "ICA FOCUS",
            "-100.00",
        );
        let r2 = dated_row(
            NaiveDate::from_ymd_opt(2026, 1, 15).unwrap(),
            "COOP",
            "-100.00",
        );
        assert_ne!(
            compute_dedup_hash_for_row("amex", &r1),
            compute_dedup_hash_for_row("amex", &r2)
        );
    }

    #[test]
    fn pending_row_differs_from_dated_row() {
        let dated = dated_row(
            NaiveDate::from_ymd_opt(2026, 1, 15).unwrap(),
            "ONLINE",
            "-50.00",
        );
        let pending = ParsedRow {
            date: None,
            description: "ONLINE".to_string(),
            amount: "-50.00".parse().unwrap(),
            currency: "SEK".to_string(),
            is_pending: true,
        };
        // "pending" is used as the date string for rows with no date.
        assert_ne!(
            compute_dedup_hash_for_row("nordea", &dated),
            compute_dedup_hash_for_row("nordea", &pending)
        );
    }
}

/// - CSV sources: `content` is raw UTF-8 text.
/// - Swedbank: `content` is the file bytes encoded as standard base64.
#[cfg(feature = "server")]
fn parse_content(
    source: &ImportSource,
    content: &str,
) -> Result<Vec<crate::csv::ParsedRow>, String> {
    match source {
        ImportSource::Amex => csv::amex::parse(content),
        ImportSource::Nordea => csv::nordea::parse(content),
        ImportSource::Ica => csv::ica::parse(content),
        ImportSource::Swedbank => {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(content)
                .map_err(|e| format!("Invalid base64 for Swedbank CSV: {e}"))?;
            csv::swedbank::parse(&bytes)
        }
    }
}

/// Preview what would be imported without modifying the database.
/// Returns counts of new / duplicate / pending rows.
///
/// For CSV sources (`Amex`, `Nordea`, `Ica`) `content` is the raw UTF-8 file text.
/// For `Swedbank` `content` is the file bytes encoded as standard base64.
#[server]
pub async fn preview_csv(
    source: ImportSource,
    content: String,
) -> Result<ImportResult, ServerFnError> {
    let household_id = current_household_id().await?;

    let rows = parse_content(&source, &content).map_err(ServerFnError::new)?;

    let db = pool();
    let source_str = source.to_string();

    let hashes: Vec<String> = rows
        .iter()
        .map(|row| compute_dedup_hash_for_row(&source_str, row))
        .collect();

    let existing: std::collections::HashSet<String> = sqlx::query_scalar(
        "SELECT dedup_hash FROM transactions WHERE household_id = $1 AND dedup_hash = ANY($2)",
    )
    .bind(household_id)
    .bind(&hashes[..])
    .fetch_all(db)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?
    .into_iter()
    .collect();

    let mut imported: u32 = 0;
    let mut skipped: u32 = 0;
    let mut pending: u32 = 0;

    for (row, hash) in rows.iter().zip(hashes.iter()) {
        if existing.contains(hash) {
            skipped += 1;
        } else if row.is_pending {
            pending += 1;
        } else {
            imported += 1;
        }
    }

    Ok(ImportResult {
        imported,
        skipped,
        pending,
    })
}

/// Import a file for the current household. Returns counts of imported / skipped / pending rows.
///
/// For CSV sources (`Amex`, `Nordea`, `Ica`) `content` is the raw UTF-8 file text.
/// For `Swedbank` `content` is the file bytes encoded as standard base64.
#[server]
pub async fn import_csv(
    source: ImportSource,
    content: String,
) -> Result<ImportResult, ServerFnError> {
    let household_id = current_household_id().await?;

    let rows = parse_content(&source, &content).map_err(ServerFnError::new)?;

    let db = pool();
    let source_str = source.to_string();
    let mut imported: u32 = 0;
    let mut skipped: u32 = 0;
    let mut pending: u32 = 0;

    for row in rows {
        let dedup_hash = compute_dedup_hash_for_row(&source_str, &row);

        let result = sqlx::query(
            r#"
            INSERT INTO transactions (date, description, amount, source, currency, dedup_hash, is_pending, household_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (household_id, dedup_hash) DO NOTHING
            "#,
        )
        .bind(row.date)
        .bind(&row.description)
        .bind(row.amount)
        .bind(&source_str)
        .bind(&row.currency)
        .bind(&dedup_hash)
        .bind(row.is_pending)
        .bind(household_id)
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

    Ok(ImportResult {
        imported,
        skipped,
        pending,
    })
}

/// Validate and normalize a manual transaction request.
#[cfg(feature = "server")]
fn normalize_create_request(
    mut req: crate::models::CreateTransactionRequest,
) -> crate::models::CreateTransactionRequest {
    req.description = req.description.trim().to_string();
    req.source = req.source.trim().to_string();
    req.currency = req.currency.trim().to_ascii_uppercase().to_string();
    req
}

/// Validate a manual transaction request, returning a human-friendly error
/// message when it is invalid.
#[cfg(feature = "server")]
fn validate_create_request(req: &crate::models::CreateTransactionRequest) -> Result<(), String> {
    if req.description.is_empty() {
        return Err("Description cannot be empty".to_string());
    }
    if req.source.is_empty() {
        return Err("Source cannot be empty".to_string());
    }
    if req.currency.is_empty() {
        return Err("Currency cannot be empty".to_string());
    }
    if req.amount.is_zero() {
        return Err("Amount cannot be zero".to_string());
    }
    Ok(())
}

/// Insert a single manually-created transaction into the current household.
#[server]
pub async fn create_transaction(
    req: crate::models::CreateTransactionRequest,
) -> Result<crate::models::Transaction, ServerFnError> {
    let household_id = current_household_id().await?;
    let req = normalize_create_request(req);
    validate_create_request(&req).map_err(ServerFnError::new)?;

    let db = pool();
    let is_pending = req.date.is_none();
    let dedup_hash = compute_dedup_hash(&req.source, req.date, &req.description, req.amount);

    let row: TransactionRow = sqlx::query_as(
        r#"
        INSERT INTO transactions (date, description, amount, source, currency, dedup_hash, is_pending, household_id)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (household_id, dedup_hash) DO NOTHING
        RETURNING
            id,
            date,
            description,
            amount,
            source,
            currency,
            is_pending,
            category_id,
            NULL::text AS category_name,
            NULL::text AS category_color,
            NULL::uuid AS category_parent_id,
            NULL::bool AS category_ignored
        "#,
    )
    .bind(req.date)
    .bind(&req.description)
    .bind(req.amount)
    .bind(&req.source)
    .bind(&req.currency)
    .bind(&dedup_hash)
    .bind(is_pending)
    .bind(household_id)
    .fetch_one(db)
    .await
    .map_err(|e| {
        if e.to_string().contains("duplicate") {
            ServerFnError::new("This transaction already exists")
        } else {
            ServerFnError::new(e.to_string())
        }
    })?;

    Ok(row.into())
}

/// Insert many manually-created transactions for the current household.
/// Returns counts of imported / skipped / pending rows. Duplicates are
/// silently skipped exactly like CSV imports.
#[server]
pub async fn create_transactions_bulk(
    reqs: Vec<crate::models::CreateTransactionRequest>,
) -> Result<ImportResult, ServerFnError> {
    let household_id = current_household_id().await?;
    let db = pool();

    let mut imported: u32 = 0;
    let mut skipped: u32 = 0;
    let mut pending: u32 = 0;

    for mut req in reqs {
        req = normalize_create_request(req);
        if validate_create_request(&req).is_err() {
            skipped += 1;
            continue;
        }

        let is_pending = req.date.is_none();
        let dedup_hash = compute_dedup_hash(&req.source, req.date, &req.description, req.amount);

        let result = sqlx::query(
            r#"
            INSERT INTO transactions (date, description, amount, source, currency, dedup_hash, is_pending, household_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (household_id, dedup_hash) DO NOTHING
            "#,
        )
        .bind(req.date)
        .bind(&req.description)
        .bind(req.amount)
        .bind(&req.source)
        .bind(&req.currency)
        .bind(&dedup_hash)
        .bind(is_pending)
        .bind(household_id)
        .execute(db)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        if result.rows_affected() == 0 {
            skipped += 1;
        } else if is_pending {
            pending += 1;
        } else {
            imported += 1;
        }
    }

    Ok(ImportResult {
        imported,
        skipped,
        pending,
    })
}

/// Fetch transactions for the current household with optional filtering.
#[server]
pub async fn get_transactions(
    filter: TransactionFilter,
) -> Result<Vec<Transaction>, ServerFnError> {
    let household_id = current_household_id().await?;
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
            c.parent_id AS category_parent_id,
            c.ignored  AS category_ignored
        FROM transactions t
        LEFT JOIN categories c ON c.id = t.category_id
        WHERE t.household_id = $1
            AND ($2::boolean = false OR t.category_id IS NULL)
            AND ($3::uuid IS NULL OR t.category_id = $3)
            AND ($4::uuid IS NULL OR EXISTS (
                SELECT 1 FROM transaction_groups tg
                WHERE tg.transaction_id = t.id AND tg.group_id = $4
            ))
            AND ($5::date IS NULL OR t.date >= $5)
            AND ($6::date IS NULL OR t.date <= $6)
            AND ($7::boolean = false OR COALESCE(c.ignored, false) = false)
        ORDER BY t.date DESC NULLS LAST, t.created_at DESC
        "#,
    )
    .bind(household_id)
    .bind(filter.unprocessed_only)
    .bind(filter.category_id)
    .bind(filter.group_id)
    .bind(filter.date_from)
    .bind(filter.date_to)
    .bind(filter.exclude_ignored)
    .fetch_all(db)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(rows.into_iter().map(Into::into).collect())
}

/// Update the mutable fields of a transaction that belongs to the current household.
/// Only the fields provided as `Some` are changed; others are left as-is.
#[server]
pub async fn update_transaction(
    req: crate::models::UpdateTransactionRequest,
) -> Result<crate::models::Transaction, ServerFnError> {
    let household_id = current_household_id().await?;
    let db = pool();

    // Fetch the current row so we can apply partial updates.
    let current: TransactionRow = sqlx::query_as(
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
            c.parent_id AS category_parent_id,
            c.ignored  AS category_ignored
        FROM transactions t
        LEFT JOIN categories c ON c.id = t.category_id
        WHERE t.id = $1 AND t.household_id = $2
        "#,
    )
    .bind(req.id)
    .bind(household_id)
    .fetch_one(db)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    let new_date = req.date.or(current.date);
    let new_description = req
        .description
        .map(|d| d.trim().to_string())
        .unwrap_or(current.description.clone());
    let new_amount = req.amount.unwrap_or(current.amount);
    let new_currency = req
        .currency
        .map(|c| c.trim().to_ascii_uppercase())
        .unwrap_or(current.currency.clone());
    let new_source = req
        .source
        .map(|s| s.trim().to_string())
        .unwrap_or(current.source.clone());

    if new_description.is_empty() {
        return Err(ServerFnError::new("Description cannot be empty"));
    }

    let new_is_pending = new_date.is_none();
    let new_dedup_hash = compute_dedup_hash(&new_source, new_date, &new_description, new_amount);

    let updated: TransactionRow = sqlx::query_as(
        r#"
        UPDATE transactions
        SET date        = $1,
            description = $2,
            amount      = $3,
            currency    = $4,
            source      = $5,
            is_pending  = $6,
            dedup_hash  = $7
        WHERE id = $8 AND household_id = $9
        RETURNING
            id,
            date,
            description,
            amount,
            source,
            currency,
            is_pending,
            category_id,
            NULL::text AS category_name,
            NULL::text AS category_color,
            NULL::uuid AS category_parent_id,
            NULL::bool AS category_ignored
        "#,
    )
    .bind(new_date)
    .bind(&new_description)
    .bind(new_amount)
    .bind(&new_currency)
    .bind(&new_source)
    .bind(new_is_pending)
    .bind(&new_dedup_hash)
    .bind(req.id)
    .bind(household_id)
    .fetch_one(db)
    .await
    .map_err(|e| {
        if e.to_string().contains("duplicate") || e.to_string().contains("unique") {
            ServerFnError::new("A transaction with identical fields already exists")
        } else {
            ServerFnError::new(e.to_string())
        }
    })?;

    Ok(updated.into())
}

/// Delete a transaction that belongs to the current household.
#[server]
pub async fn delete_transaction(id: uuid::Uuid) -> Result<(), ServerFnError> {
    let household_id = current_household_id().await?;
    let db = pool();

    sqlx::query("DELETE FROM transactions WHERE id = $1 AND household_id = $2")
        .bind(id)
        .bind(household_id)
        .execute(db)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}

/// Return the next unclassified (non-pending) transaction for the current household
/// and the total remaining count. The queue is ordered oldest-first.
#[server]
pub async fn get_queue_state() -> Result<QueueState, ServerFnError> {
    let household_id = current_household_id().await?;
    let db = pool();

    let remaining: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM transactions \
         WHERE household_id = $1 AND category_id IS NULL AND is_pending = false",
    )
    .bind(household_id)
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
            c.parent_id AS category_parent_id,
            c.ignored  AS category_ignored
        FROM transactions t
        LEFT JOIN categories c ON c.id = t.category_id
        WHERE t.household_id = $1 AND t.category_id IS NULL AND t.is_pending = false
        ORDER BY t.date ASC NULLS LAST, t.created_at ASC
        LIMIT 4
        "#,
    )
    .bind(household_id)
    .fetch_all(db)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    let mut items: Vec<Transaction> = rows.into_iter().map(Into::into).collect();
    let next = if items.is_empty() {
        None
    } else {
        Some(items.remove(0))
    };
    let upcoming = items;

    Ok(QueueState {
        next,
        upcoming,
        remaining,
    })
}
