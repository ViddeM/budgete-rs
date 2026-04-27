use crate::models::DashboardStats;
use dioxus::prelude::*;

#[cfg(feature = "server")]
use {
    crate::auth::session::current_user_id,
    crate::db::pool,
    crate::db_rows::{CategorySpendRow, CountRow, PrevTotalsRow, TotalsRow},
    crate::models::CategorySpend,
    chrono::Datelike,
    rust_decimal::Decimal,
};

/// Compute dashboard statistics for the current calendar month, scoped to the
/// current user.
#[server]
pub async fn get_dashboard_stats() -> Result<DashboardStats, ServerFnError> {
    let user_id = current_user_id().await?;
    let db = pool();

    let today = chrono::Local::now().date_naive();
    let month_start =
        chrono::NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
    let (prev_year, prev_month) = if today.month() == 1 {
        (today.year() - 1, 12u32)
    } else {
        (today.year(), today.month() - 1)
    };
    let prev_month_start =
        chrono::NaiveDate::from_ymd_opt(prev_year, prev_month, 1).unwrap();

    // Totals this month
    let totals: TotalsRow = sqlx::query_as(
        r#"
        SELECT
            COALESCE(SUM(amount) FILTER (WHERE amount < 0), 0) AS expenses,
            COALESCE(SUM(amount) FILTER (WHERE amount > 0), 0) AS income
        FROM transactions
        WHERE user_id = $1 AND date >= $2 AND is_pending = false
        "#,
    )
    .bind(user_id)
    .bind(month_start)
    .fetch_one(db)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    // Previous month expenses
    let prev_totals: PrevTotalsRow = sqlx::query_as(
        r#"
        SELECT
            COALESCE(SUM(amount) FILTER (WHERE amount < 0), 0) AS expenses
        FROM transactions
        WHERE user_id = $1 AND date >= $2 AND date < $3 AND is_pending = false
        "#,
    )
    .bind(user_id)
    .bind(prev_month_start)
    .bind(month_start)
    .fetch_one(db)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    // Unprocessed count
    let unprocessed: CountRow = sqlx::query_as(
        "SELECT COUNT(*) AS count FROM transactions \
         WHERE user_id = $1 AND category_id IS NULL AND is_pending = false",
    )
    .bind(user_id)
    .fetch_one(db)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    // Top 5 categories this month
    let cat_rows: Vec<CategorySpendRow> = sqlx::query_as(
        r#"
        SELECT
            c.id,
            c.name,
            c.color,
            ABS(SUM(t.amount)) AS total
        FROM transactions t
        JOIN categories c ON c.id = t.category_id
        WHERE t.user_id = $1 AND t.date >= $2 AND t.amount < 0 AND t.is_pending = false
        GROUP BY c.id, c.name, c.color
        ORDER BY total DESC
        LIMIT 5
        "#,
    )
    .bind(user_id)
    .bind(month_start)
    .fetch_all(db)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    let top_categories = cat_rows
        .into_iter()
        .map(|r| CategorySpend {
            category_id: r.id,
            category_name: r.name,
            category_color: r.color,
            total: r.total,
        })
        .collect();

    let mom_delta_pct = {
        let curr = totals.expenses;
        let prev = prev_totals.expenses;
        if prev.is_zero() {
            None
        } else {
            Some((curr - prev) / prev.abs() * Decimal::ONE_HUNDRED)
        }
    };

    Ok(DashboardStats {
        month_expenses: totals.expenses.abs(),
        month_income: totals.income,
        unprocessed_count: unprocessed.count,
        top_categories,
        mom_delta_pct,
    })
}
