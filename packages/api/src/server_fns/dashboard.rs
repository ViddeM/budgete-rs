use crate::models::{CategorySpend, DashboardStats, SpendingOverTime};
use dioxus::prelude::*;

#[cfg(feature = "server")]
use {
    crate::db::pool,
    crate::db_rows::{CategorySpendRow, CountRow, OverTimeRow, PrevTotalsRow, TotalsRow},
    chrono::Datelike,
    rust_decimal::Decimal,
};

/// Compute dashboard statistics for the current calendar month.
#[server]
pub async fn get_dashboard_stats() -> Result<DashboardStats, ServerFnError> {
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
        WHERE date >= $1 AND is_pending = false
        "#,
    )
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
        WHERE date >= $1 AND date < $2 AND is_pending = false
        "#,
    )
    .bind(prev_month_start)
    .bind(month_start)
    .fetch_one(db)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    // Unprocessed count
    let unprocessed: CountRow = sqlx::query_as(
        "SELECT COUNT(*) AS count FROM transactions WHERE category_id IS NULL AND is_pending = false",
    )
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
        WHERE t.date >= $1 AND t.amount < 0 AND t.is_pending = false
        GROUP BY c.id, c.name, c.color
        ORDER BY total DESC
        LIMIT 5
        "#,
    )
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

/// Spending broken down by category for a date range.
#[server]
pub async fn get_spending_by_category(
    date_from: chrono::NaiveDate,
    date_to: chrono::NaiveDate,
) -> Result<Vec<CategorySpend>, ServerFnError> {
    let db = pool();

    let rows: Vec<CategorySpendRow> = sqlx::query_as(
        r#"
        SELECT
            c.id,
            c.name,
            c.color,
            ABS(SUM(t.amount)) AS total
        FROM transactions t
        JOIN categories c ON c.id = t.category_id
        WHERE t.date >= $1 AND t.date <= $2 AND t.amount < 0 AND t.is_pending = false
        GROUP BY c.id, c.name, c.color
        ORDER BY total DESC
        "#,
    )
    .bind(date_from)
    .bind(date_to)
    .fetch_all(db)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(rows
        .into_iter()
        .map(|r| CategorySpend {
            category_id: r.id,
            category_name: r.name,
            category_color: r.color,
            total: r.total,
        })
        .collect())
}

/// Spending and income aggregated by month for a date range.
#[server]
pub async fn get_spending_over_time(
    date_from: chrono::NaiveDate,
    date_to: chrono::NaiveDate,
) -> Result<Vec<SpendingOverTime>, ServerFnError> {
    let db = pool();

    let rows: Vec<OverTimeRow> = sqlx::query_as(
        r#"
        SELECT
            TO_CHAR(date_trunc('month', date), 'YYYY-MM') AS period_label,
            ABS(COALESCE(SUM(amount) FILTER (WHERE amount < 0), 0)) AS expenses,
            COALESCE(SUM(amount) FILTER (WHERE amount > 0), 0) AS income
        FROM transactions
        WHERE date >= $1 AND date <= $2 AND is_pending = false
        GROUP BY date_trunc('month', date)
        ORDER BY date_trunc('month', date)
        "#,
    )
    .bind(date_from)
    .bind(date_to)
    .fetch_all(db)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(rows
        .into_iter()
        .map(|r| SpendingOverTime {
            period_label: r.period_label,
            expenses: r.expenses,
            income: r.income,
        })
        .collect())
}
