use crate::models::{CategorySpend, SpendingOverTime};
use chrono::NaiveDate;
use dioxus::prelude::*;
use uuid::Uuid;

#[cfg(feature = "server")]
use {
    crate::auth::session::current_user_id,
    crate::db::pool,
    crate::db_rows::{CategorySpendRow, OverTimeRow},
};

/// Total expense per category for the current user in the given date range,
/// optionally scoped to a project (group). Returns absolute (positive) expense
/// values, largest first.
#[server]
pub async fn get_spending_by_category(
    date_from: NaiveDate,
    date_to: NaiveDate,
    group_id: Option<Uuid>,
) -> Result<Vec<CategorySpend>, ServerFnError> {
    let user_id = current_user_id().await?;
    let db = pool();

    let rows: Vec<CategorySpendRow> = sqlx::query_as(
        r#"
        SELECT
            c.id,
            c.name,
            c.color,
            SUM(-t.amount) AS total
        FROM transactions t
        JOIN categories c ON t.category_id = c.id
        WHERE t.user_id = $1
          AND t.date >= $2
          AND t.date <= $3
          AND t.is_pending = false
          AND t.amount < 0
          AND ($4::uuid IS NULL OR EXISTS (
                SELECT 1 FROM transaction_groups tg
                WHERE tg.transaction_id = t.id AND tg.group_id = $4
              ))
        GROUP BY c.id, c.name, c.color
        ORDER BY SUM(-t.amount) DESC
        "#,
    )
    .bind(user_id)
    .bind(date_from)
    .bind(date_to)
    .bind(group_id)
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

/// Expenses and income for the current user aggregated by calendar month for
/// the given date range, optionally scoped to a project. Both `expenses` and
/// `income` are positive values.
#[server]
pub async fn get_spending_over_time(
    date_from: NaiveDate,
    date_to: NaiveDate,
    group_id: Option<Uuid>,
) -> Result<Vec<SpendingOverTime>, ServerFnError> {
    let user_id = current_user_id().await?;
    let db = pool();

    let rows: Vec<OverTimeRow> = sqlx::query_as(
        r#"
        SELECT
            TO_CHAR(DATE_TRUNC('month', t.date), 'YYYY-MM') AS period_label,
            SUM(CASE WHEN t.amount < 0 THEN -t.amount ELSE 0::numeric END) AS expenses,
            SUM(CASE WHEN t.amount > 0 THEN  t.amount ELSE 0::numeric END) AS income
        FROM transactions t
        WHERE t.user_id = $1
          AND t.date >= $2
          AND t.date <= $3
          AND t.is_pending = false
          AND ($4::uuid IS NULL OR EXISTS (
                SELECT 1 FROM transaction_groups tg
                WHERE tg.transaction_id = t.id AND tg.group_id = $4
              ))
        GROUP BY DATE_TRUNC('month', t.date)
        ORDER BY DATE_TRUNC('month', t.date)
        "#,
    )
    .bind(user_id)
    .bind(date_from)
    .bind(date_to)
    .bind(group_id)
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
