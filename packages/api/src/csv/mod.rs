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
pub mod ica;
pub mod klarna;
pub mod nordea;
pub mod swedbank;

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

#[cfg(test)]
mod tests {
    use super::parse_swedish_decimal;

    #[test]
    fn basic_decimal() {
        assert_eq!(parse_swedish_decimal("35,00").unwrap().to_string(), "35.00");
    }

    #[test]
    fn thousands_separator() {
        assert_eq!(parse_swedish_decimal("1 234,56").unwrap().to_string(), "1234.56");
    }

    #[test]
    fn negative_ascii_minus() {
        assert_eq!(parse_swedish_decimal("-175,00").unwrap().to_string(), "-175.00");
    }

    #[test]
    fn negative_unicode_minus() {
        assert_eq!(parse_swedish_decimal("\u{2212}175,00").unwrap().to_string(), "-175.00");
    }

    #[test]
    fn negative_with_thousands() {
        assert_eq!(parse_swedish_decimal("-1 500,00").unwrap().to_string(), "-1500.00");
    }

    #[test]
    fn integer_no_decimal() {
        assert_eq!(parse_swedish_decimal("1000").unwrap().to_string(), "1000");
    }

    #[test]
    fn leading_trailing_whitespace_trimmed() {
        assert_eq!(parse_swedish_decimal("  42,50  ").unwrap().to_string(), "42.50");
    }

    #[test]
    fn invalid_input_errors() {
        assert!(parse_swedish_decimal("not-a-number").is_err());
    }

    #[test]
    fn empty_string_errors() {
        assert!(parse_swedish_decimal("").is_err());
    }
}
