use super::{parse_swedish_decimal, ParsedRow};
use chrono::NaiveDate;

/// Parse an Amex CSV export.
///
/// Format: `Datum,Beskrivning,Belopp`
/// - Date: MM/DD/YYYY
/// - Separator: `,`
/// - Amount: Swedish decimal comma, positive = charge → we flip to negative.
///   The one exception is a negative Amex row (e.g. "BETALNING MOTTAGEN") which
///   is a credit → becomes positive (income).
pub fn parse(content: &str) -> Result<Vec<ParsedRow>, String> {
    let mut rows = Vec::new();

    for (line_no, line) in content.lines().enumerate() {
        // Skip header
        if line_no == 0 {
            continue;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Amex wraps the amount field in quotes when it contains a comma, e.g.:
        // 04/23/2026,ICA FOCUS,"112,54"
        // We do a simple field split that respects double-quoted fields.
        let fields = split_csv_line(line);
        if fields.len() < 3 {
            return Err(format!("Amex line {line_no}: expected 3 fields, got {}", fields.len()));
        }

        let date_str = fields[0].trim().trim_matches('"');
        let description = fields[1].trim().trim_matches('"').to_string();
        let amount_str = fields[2].trim().trim_matches('"');

        let date = NaiveDate::parse_from_str(date_str, "%m/%d/%Y")
            .map_err(|e| format!("Amex line {line_no}: bad date '{date_str}': {e}"))?;

        let raw = parse_swedish_decimal(amount_str)
            .map_err(|e| format!("Amex line {line_no}: bad amount '{amount_str}': {e}"))?;

        // Amex: positive amount = charge (expense) → flip to negative.
        // Negative amount = payment received (credit) → flip to positive.
        let amount = -raw;

        rows.push(ParsedRow {
            date: Some(date),
            description,
            amount,
            currency: "SEK".to_string(),
            is_pending: false,
        });
    }

    Ok(rows)
}

/// Split a CSV line, respecting double-quoted fields.
fn split_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in line.chars() {
        match ch {
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                fields.push(current.clone());
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    fields.push(current);
    fields
}
