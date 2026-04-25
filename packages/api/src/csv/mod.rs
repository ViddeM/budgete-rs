use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::str::FromStr;

/// A raw parsed row before dedup / DB insertion.
#[derive(Debug)]
pub struct ParsedRow {
    pub date: Option<NaiveDate>,
    pub description: String,
    pub amount: Decimal,
    pub currency: String,
    pub is_pending: bool,
}

pub mod amex;
pub mod nordea;

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Parse a Swedish decimal string like "1 234,56" or "1234,56" or "-175,00".
/// Also handles the Unicode minus '−' (U+2212).
pub fn parse_swedish_decimal(s: &str) -> Result<Decimal, rust_decimal::Error> {
    // Replace Unicode minus with ASCII minus, strip spaces, replace comma decimal separator
    let normalised = s
        .trim()
        .replace('\u{2212}', "-") // Unicode minus sign
        .replace(' ', "")
        .replace(',', ".");
    Decimal::from_str(&normalised)
}
