use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Core domain types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Category {
    pub id: Uuid,
    pub name: String,
    pub color: String,
    /// `None` for top-level categories; `Some(parent_id)` for subcategories.
    pub parent_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Group {
    pub id: Uuid,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Transaction {
    pub id: Uuid,
    pub date: Option<NaiveDate>,
    pub description: String,
    /// Negative = expense, positive = income. SEK.
    pub amount: Decimal,
    pub source: String,
    pub currency: String,
    pub is_pending: bool,
    /// `None` when the transaction has not yet been classified.
    pub category: Option<Category>,
}

// ---------------------------------------------------------------------------
// Server function I/O types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImportResult {
    pub imported: u32,
    pub skipped: u32,
    pub pending: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TransactionFilter {
    pub unprocessed_only: bool,
    pub category_id: Option<Uuid>,
    pub group_id: Option<Uuid>,
    pub date_from: Option<NaiveDate>,
    pub date_to: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DashboardStats {
    /// Total expenses this calendar month (negative value → stored as positive for display)
    pub month_expenses: Decimal,
    /// Total income this calendar month
    pub month_income: Decimal,
    /// Number of transactions with category_id IS NULL
    pub unprocessed_count: i64,
    /// Top 5 categories by spend this month
    pub top_categories: Vec<CategorySpend>,
    /// Month-over-month expense delta percentage
    pub mom_delta_pct: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CategorySpend {
    pub category_id: Uuid,
    pub category_name: String,
    pub category_color: String,
    pub total: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpendingOverTime {
    pub period_label: String,
    pub expenses: Decimal,
    pub income: Decimal,
}

/// State returned by the classify queue endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QueueState {
    /// The oldest unclassified, non-pending transaction; `None` when the queue is empty.
    pub next: Option<Transaction>,
    /// The next few transactions after `next` (read-only preview, up to 3).
    pub upcoming: Vec<Transaction>,
    /// Total number of unclassified, non-pending transactions (including `next`).
    pub remaining: i64,
}

/// The two supported CSV import sources.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CsvSource {
    Amex,
    Nordea,
}

impl std::fmt::Display for CsvSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CsvSource::Amex => write!(f, "amex"),
            CsvSource::Nordea => write!(f, "nordea"),
        }
    }
}
