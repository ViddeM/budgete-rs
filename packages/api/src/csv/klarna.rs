use super::{parse_swedish_decimal, ParsedRow};
use chrono::NaiveDate;

/// Parse a Klarna Monthly invoice PDF.
///
/// The PDF contains a table with columns: DATE | DESCRIPTION | PAYMENT METHOD | AMOUNT
/// - Date: YYYY-MM-DD (ISO 8601)
/// - Amount: Swedish decimal, positive = charge → we flip to negative (same convention as Amex).
/// - Currency is always SEK; there are no pending rows in the monthly invoice.
///
/// `pdf_bytes` is the raw binary content of the PDF file.
#[cfg(feature = "server")]
pub fn parse(pdf_bytes: &[u8]) -> Result<Vec<ParsedRow>, String> {
    let text = pdf_extract::extract_text_from_mem(pdf_bytes)
        .map_err(|e| format!("Failed to extract text from Klarna PDF: {e}"))?;
    parse_text(&text)
}

/// Parse the extracted plain text from the Klarna PDF.
///
/// `pdf-extract` renders each transaction as a **single line**:
///
///   `"2026-06-13 U Uber Klarna card 144,00 kr"`
///
/// The column header is similarly on one line:
///
///   `"DATE DESCRIPTION PAYMENT METHOD AMOUNT"`
///
/// Page-break noise lines look like: `"Date sent: 2026-06-15 Pg. 3/4"`
///
/// Strategy:
/// 1. Find the column-header sentinel to confirm we have a valid Klarna PDF.
/// 2. Scan every subsequent non-empty line until "Summary".
/// 3. For each line, attempt to parse as a transaction (starts with YYYY-MM-DD).
fn parse_text(text: &str) -> Result<Vec<ParsedRow>, String> {
    const HEADER: &str = "DATE DESCRIPTION PAYMENT METHOD AMOUNT";

    // Confirm this looks like a Klarna invoice.
    if !text.contains(HEADER) {
        return Err("Could not find transaction table header in Klarna PDF".to_string());
    }

    let mut rows: Vec<ParsedRow> = Vec::new();
    let mut in_table = false;

    for line in text.lines() {
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        // Enter the transaction table on the column-header line.
        if line == HEADER {
            in_table = true;
            continue;
        }

        if !in_table {
            continue;
        }

        // Stop at the summary footer.
        if line == "Summary" {
            break;
        }

        // Skip repeated page-break noise: "Date sent: … Pg. N/M"
        if line.starts_with("Date sent:") {
            continue;
        }

        // Each transaction line starts with an ISO date.
        if let Some(row) = parse_transaction_line(line) {
            rows.push(row);
        }
    }

    Ok(rows)
}

/// Attempt to parse a single transaction line.
///
/// Format: `YYYY-MM-DD <icon> <description> <payment method> <amount> kr`
///
/// - The icon is a single character (letter or digit) immediately after the date.
/// - The amount ends with ` kr` and may contain spaces (Swedish thousands separator).
/// - Payment method is one of "Klarna card" or "Pay later" (two tokens).
///   We strip it from the right after the amount.
/// - Everything between the icon and the payment method is the description.
fn parse_transaction_line(line: &str) -> Option<ParsedRow> {
    // Must start with YYYY-MM-DD
    if line.len() < 10 {
        return None;
    }
    let date = NaiveDate::parse_from_str(&line[..10], "%Y-%m-%d").ok()?;

    // Rest of line after the date and a space.
    let rest = line.get(11..)?.trim();

    // Strip " kr" suffix and parse the amount (may include spaces as thousands sep).
    // The amount itself can be e.g. "144,00" or "1 399,00" so we work from the right.
    let rest = rest.strip_suffix(" kr")?;

    // Amount: last whitespace-delimited token, BUT Swedish amounts can have a
    // space as a thousands separator ("1 399,00"). We find the amount by scanning
    // right-to-left for the comma that marks the decimal separator.
    let amount_str = extract_amount_from_end(rest)?;
    let amount_end = rest.len() - amount_str.len();
    let before_amount = rest[..amount_end].trim_end();

    let raw_amount = parse_swedish_decimal(amount_str).ok()?;

    // Strip payment method from right: "Klarna card" or "Pay later"
    let before_pm = strip_payment_method(before_amount)?;
    let before_pm = before_pm.trim_end();

    // Strip the single-character icon from left.
    let mut chars = before_pm.chars();
    let _icon = chars.next()?; // single icon character
    let description = chars.as_str().trim().to_string();

    if description.is_empty() {
        return None;
    }

    Some(ParsedRow {
        date: Some(date),
        description,
        // Klarna invoice shows positive charges → flip to negative (expense).
        amount: -raw_amount,
        currency: "SEK".to_string(),
        is_pending: false,
    })
}

/// Extract the numeric amount string (everything after the last space that
/// precedes the decimal comma section) from the end of a string.
///
/// Swedish amounts: "144,00" or "1 399,00" or "109,11"
/// We scan right-to-left: the decimal part is everything after the last `,`,
/// and the integer part may include spaces.
fn extract_amount_from_end(s: &str) -> Option<&str> {
    // Find the rightmost comma — that separates integer from decimal part.
    let comma_pos = s.rfind(',')?;

    // Walk left from the comma to find where the amount starts:
    // stop at the second space (amounts like "1 399" have exactly one internal space).
    let prefix = &s[..comma_pos];
    let amount_start = if let Some(space_pos) = prefix.rfind(' ') {
        // Check if the character before that space is a digit (thousands sep space)
        // vs a separator between fields.
        let before_space = &prefix[..space_pos];
        if before_space.ends_with(|c: char| c.is_ascii_digit()) {
            // Could be thousands separator — check for another space before that.
            if let Some(prev_space) = before_space.rfind(' ') {
                prev_space + 1
            } else {
                space_pos + 1
            }
        } else {
            space_pos + 1
        }
    } else {
        0
    };

    Some(&s[amount_start..])
}

/// Strip a known Klarna payment method suffix from the right of a string.
fn strip_payment_method(s: &str) -> Option<&str> {
    for pm in ["Klarna card", "Pay later"] {
        if let Some(stripped) = s.strip_suffix(pm) {
            return Some(stripped);
        }
    }
    // Unknown payment method — not a valid transaction line.
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Representative lines matching the actual pdf-extract output format.
    const SAMPLE: &str = r#"
Date sent: 2026-06-15

 Pg. 1/4

DATE DESCRIPTION PAYMENT METHOD AMOUNT

2026-06-13 U Uber Klarna card 144,00 kr

2026-06-04 N Naturkompaniet Klarna card 1 399,00 kr

2026-06-02 S Skicka blommor med Blomsterlandet Pay later 536,00 kr

2026-06-04 S Steam Klarna card 109,11 kr

Date sent: 2026-06-15 Pg. 3/4

2026-06-03 M Maxi ICA Stormarknad Klarna card 142,26 kr

Summary

Total orders (5) 2 330,37 kr
"#;

    #[test]
    fn test_parse_text_sample() {
        let rows = parse_text(SAMPLE).expect("parse should succeed");
        assert_eq!(rows.len(), 5);

        assert_eq!(rows[0].description, "Uber");
        assert_eq!(rows[0].date, NaiveDate::from_ymd_opt(2026, 6, 13));
        assert_eq!(rows[0].amount.to_string(), "-144.00");

        assert_eq!(rows[1].description, "Naturkompaniet");
        assert_eq!(rows[1].amount.to_string(), "-1399.00");

        assert_eq!(rows[2].description, "Skicka blommor med Blomsterlandet");
        assert_eq!(rows[2].amount.to_string(), "-536.00");

        assert_eq!(rows[3].description, "Steam");
        assert_eq!(rows[3].amount.to_string(), "-109.11");

        assert_eq!(rows[4].description, "Maxi ICA Stormarknad");
        assert_eq!(rows[4].amount.to_string(), "-142.26");

        for row in &rows {
            assert_eq!(row.currency, "SEK");
            assert!(!row.is_pending);
        }
    }

    #[test]
    fn test_page_break_mid_table() {
        let text = concat!(
            "DATE DESCRIPTION PAYMENT METHOD AMOUNT\n",
            "2026-06-13 U Uber Klarna card 144,00 kr\n",
            "Date sent: 2026-06-15 Pg. 3/4\n",
            "DATE DESCRIPTION PAYMENT METHOD AMOUNT\n",
            "2026-06-04 N Naturkompaniet Klarna card 1 399,00 kr\n",
            "Summary\n",
        );
        let rows = parse_text(text).expect("parse should succeed");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].description, "Uber");
        assert_eq!(rows[1].description, "Naturkompaniet");
    }

    #[test]
    fn test_no_header_returns_error() {
        let result = parse_text("some random text without the header");
        assert!(result.is_err());
    }
}
