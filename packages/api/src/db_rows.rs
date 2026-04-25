use crate::models::{Category, Group, Transaction};

// ---------------------------------------------------------------------------
// Row types for sqlx::query_as — used to avoid needing offline query data.
// These are server-only and live in this module.
// ---------------------------------------------------------------------------

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
pub(crate) struct CategoryRow {
    pub id: uuid::Uuid,
    pub name: String,
    pub color: String,
    pub parent_id: Option<uuid::Uuid>,
}

#[cfg(feature = "server")]
impl From<CategoryRow> for Category {
    fn from(r: CategoryRow) -> Self {
        Category { id: r.id, name: r.name, color: r.color, parent_id: r.parent_id }
    }
}

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
pub(crate) struct GroupRow {
    pub id: uuid::Uuid,
    pub name: String,
    pub description: String,
}

#[cfg(feature = "server")]
impl From<GroupRow> for Group {
    fn from(r: GroupRow) -> Self {
        Group { id: r.id, name: r.name, description: r.description }
    }
}

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
pub(crate) struct TransactionRow {
    pub id: uuid::Uuid,
    pub date: Option<chrono::NaiveDate>,
    pub description: String,
    pub amount: rust_decimal::Decimal,
    pub source: String,
    pub currency: String,
    pub is_pending: bool,
    pub category_id: Option<uuid::Uuid>,
    pub category_name: Option<String>,
    pub category_color: Option<String>,
}

#[cfg(feature = "server")]
impl From<TransactionRow> for Transaction {
    fn from(r: TransactionRow) -> Self {
        Transaction {
            id: r.id,
            date: r.date,
            description: r.description,
            amount: r.amount,
            source: r.source,
            currency: r.currency,
            is_pending: r.is_pending,
            category_id: r.category_id,
            category_name: r.category_name,
            category_color: r.category_color,
        }
    }
}

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
pub(crate) struct TotalsRow {
    pub expenses: rust_decimal::Decimal,
    pub income: rust_decimal::Decimal,
}

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
pub(crate) struct PrevTotalsRow {
    pub expenses: rust_decimal::Decimal,
}

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
pub(crate) struct CountRow {
    pub count: i64,
}

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
pub(crate) struct CategorySpendRow {
    pub id: uuid::Uuid,
    pub name: String,
    pub color: String,
    pub total: rust_decimal::Decimal,
}

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
pub(crate) struct OverTimeRow {
    pub period_label: String,
    pub expenses: rust_decimal::Decimal,
    pub income: rust_decimal::Decimal,
}
