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
            return Err(format!(
                "Amex line {line_no}: expected 3 fields, got {}",
                fields.len()
            ));
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

/// Split a CSV line, respecting double-quoted fields (quote chars are stripped).
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

#[cfg(test)]
mod tests {
    use super::*;

    // Amex quotes the amount whenever it contains a comma (Swedish decimal).
    const SAMPLE: &str = "\
Datum,Beskrivning,Belopp
04/23/2026,ICA FOCUS,\"100,00\"
05/01/2026,BETALNING MOTTAGEN,\"-1 000,00\"
05/15/2026,\"Restaurang Le Bistro\",\"112,54\"
";

    #[test]
    fn test_parse_sample() {
        let rows = parse(SAMPLE).expect("parse should succeed");
        assert_eq!(rows.len(), 3);

        // Positive Amex charge → flipped to negative expense.
        assert_eq!(rows[0].description, "ICA FOCUS");
        assert_eq!(rows[0].date, NaiveDate::from_ymd_opt(2026, 4, 23));
        assert_eq!(rows[0].amount.to_string(), "-100.00");
        assert_eq!(rows[0].currency, "SEK");
        assert!(!rows[0].is_pending);

        // Negative Amex row (credit/payment) → flipped to positive income.
        assert_eq!(rows[1].description, "BETALNING MOTTAGEN");
        assert_eq!(rows[1].amount.to_string(), "1000.00");

        // Quoted description with amount "112,54".
        assert_eq!(rows[2].description, "Restaurang Le Bistro");
        assert_eq!(rows[2].amount.to_string(), "-112.54");
    }

    #[test]
    fn test_empty_after_header() {
        let rows = parse("Datum,Beskrivning,Belopp\n").expect("parse should succeed");
        assert!(rows.is_empty());
    }

    #[test]
    fn test_too_few_fields_returns_error() {
        let csv = "Datum,Beskrivning,Belopp\n04/23/2026,ICA FOCUS\n";
        assert!(parse(csv).is_err());
    }

    #[test]
    fn test_split_simple_fields() {
        assert_eq!(
            split_csv_line("a,b,c"),
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn test_split_quoted_field_with_comma() {
        // Quotes are stripped; the internal comma is kept as part of the field value.
        assert_eq!(
            split_csv_line("\"a,b\",c"),
            vec!["a,b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn test_split_empty_middle_field() {
        assert_eq!(
            split_csv_line("a,,c"),
            vec!["a".to_string(), "".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn test_split_amex_quoted_amount() {
        // Real Amex export pattern: date,desc,"amount,with,decimal,comma"
        assert_eq!(
            split_csv_line("04/23/2026,ICA FOCUS,\"112,54\""),
            vec![
                "04/23/2026".to_string(),
                "ICA FOCUS".to_string(),
                "112,54".to_string(),
            ]
        );
    }
}
