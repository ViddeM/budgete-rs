use super::ParsedRow;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::str::FromStr;

/// Parse a Swedbank CSV export.
///
/// The file is Windows-1252 (Latin-1) encoded with CRLF line endings.
/// `raw_bytes` must be the undecoded file bytes — decoding to UTF-8 happens here.
///
/// Format (comma-separated, quoted strings):
/// ```text
/// * <metadata line>
/// Radnummer,Clearingnummer,Kontonummer,Produkt,Valuta,Bokföringsdag,Transaktionsdag,
///   Valutadag,Referens,Beskrivning,Belopp,Bokfört saldo
/// <data rows…>
/// ```
///
/// Column indices (0-based):
/// - 4  `Valuta`       — currency code (e.g. `SEK`)
/// - 5  `Bokföringsdag` — booking date, `YYYY-MM-DD`
/// - 9  `Beskrivning`   — description (may be quoted)
/// - 10 `Belopp`        — amount, standard `.` decimal, already signed correctly
///
/// The first line (`*`) is a metadata comment and is skipped.
/// The second line is the column header and is also skipped.
#[cfg(feature = "server")]
pub fn parse(raw_bytes: &[u8]) -> Result<Vec<ParsedRow>, String> {
    // Decode Windows-1252 → UTF-8, replacing unmappable bytes with the replacement char.
    let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(raw_bytes);
    parse_text(&decoded)
}

fn parse_text(content: &str) -> Result<Vec<ParsedRow>, String> {
    let mut rows = Vec::new();

    for (line_no, line) in content.lines().enumerate() {
        let line = line.trim();

        // Line 0: metadata comment ("* Transaktioner Period …") — skip.
        if line_no == 0 {
            continue;
        }
        // Line 1: column header — skip.
        if line_no == 1 {
            continue;
        }

        if line.is_empty() {
            continue;
        }

        let fields = split_csv_line(line);
        if fields.len() < 11 {
            return Err(format!(
                "Swedbank line {line_no}: expected at least 11 fields, got {}",
                fields.len()
            ));
        }

        let currency = fields[4].trim().trim_matches('"').to_string();
        let date_str = fields[5].trim().trim_matches('"');
        let description = fields[9].trim().trim_matches('"').to_string();
        let amount_str = fields[10].trim().trim_matches('"');

        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .map_err(|e| format!("Swedbank line {line_no}: bad date '{date_str}': {e}"))?;

        // Swedbank amounts use standard '.' decimal separator (not Swedish comma).
        let amount = Decimal::from_str(amount_str)
            .map_err(|e| format!("Swedbank line {line_no}: bad amount '{amount_str}': {e}"))?;

        rows.push(ParsedRow {
            date: Some(date),
            description,
            // Swedbank amounts are already signed correctly (negative = expense).
            amount,
            currency,
            is_pending: false,
        });
    }

    Ok(rows)
}

/// Split a comma-separated line, respecting double-quoted fields.
/// Quoted fields may contain commas but not escaped quotes (not present in Swedbank exports).
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

    // UTF-8 equivalent of the actual Windows-1252 CSV content (for unit tests we
    // use parse_text directly to avoid encoding complexity in test data).
    const SAMPLE: &str = "\
* Transaktioner Period 2025-01-01\u{2013}2025-12-31 Skapad 2026-06-24 18:29 CEST\r
Radnummer,Clearingnummer,Kontonummer,Produkt,Valuta,Bokföringsdag,Transaktionsdag,Valutadag,Referens,Beskrivning,Belopp,Bokfört saldo\r
1,82990,7341067226,\"Privatkonto\",SEK,2025-01-13,2025-01-13,2025-01-13,\"För lgh\",\"För lgh\",-217468.00,0.00\r
2,82990,7341067226,\"Privatkonto\",SEK,2025-01-13,2025-01-12,2025-01-12,\"829908145252717\",\"Överföring via internet\",100000.00,217468.00\r
3,82990,7341067226,\"Privatkonto\",SEK,2025-01-13,2025-01-12,2025-01-12,\"829908145252717\",\"Överföring via internet\",17468.00,117468.00\r
";

    #[test]
    fn test_parse_sample() {
        let rows = parse_text(SAMPLE).expect("parse should succeed");
        assert_eq!(rows.len(), 3);

        // Expense: negative as-is.
        assert_eq!(rows[0].description, "För lgh");
        assert_eq!(rows[0].date, NaiveDate::from_ymd_opt(2025, 1, 13));
        assert_eq!(rows[0].amount.to_string(), "-217468.00");
        assert_eq!(rows[0].currency, "SEK");

        // Income: positive as-is.
        assert_eq!(rows[1].description, "Överföring via internet");
        assert_eq!(rows[1].amount.to_string(), "100000.00");

        assert_eq!(rows[2].amount.to_string(), "17468.00");

        for row in &rows {
            assert!(!row.is_pending);
        }
    }

    #[test]
    fn test_empty_after_headers() {
        let text = "* meta\r\nheader row\r\n";
        let rows = parse_text(text).expect("parse should succeed");
        assert!(rows.is_empty());
    }
}
