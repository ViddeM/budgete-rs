use super::{parse_swedish_decimal, ParsedRow};
use chrono::NaiveDate;

/// Parse a Nordea CSV export.
///
/// Format: `Bokföringsdag;Belopp;Avsändare;Mottagare;Namn;Rubrik;Saldo;Valuta;`
/// - Date: YYYY/MM/DD  (or "Reserverat" for pending)
/// - Separator: `;`
/// - Amount: Swedish decimal comma, negative = debit (kept as-is).
/// - Currency: last non-empty field.
/// - Description: `Rubrik` column (index 5), falls back to `Namn` (index 4).
///
/// "Reserverat" rows → `is_pending = true`, `date = None`.
pub fn parse(content: &str) -> Result<Vec<ParsedRow>, String> {
    let mut rows = Vec::new();

    for (line_no, line) in content.lines().enumerate() {
        // The file may start with a UTF-8 BOM; strip it on the header line.
        let line = line.trim_start_matches('\u{feff}').trim();

        // Skip header
        if line_no == 0 {
            continue;
        }
        if line.is_empty() {
            continue;
        }

        let fields: Vec<&str> = line.split(';').collect();
        // Expected at least 8 columns (trailing semicolon gives an empty 9th)
        if fields.len() < 8 {
            return Err(format!(
                "Nordea line {line_no}: expected ≥8 fields, got {}",
                fields.len()
            ));
        }

        let date_str = fields[0].trim();
        let amount_str = fields[1].trim();
        let name = fields[4].trim();
        let rubrik = fields[5].trim();
        let currency = fields[7].trim().to_string();

        // Description: prefer Rubrik, fall back to Namn
        let description = if !rubrik.is_empty() {
            rubrik.to_string()
        } else {
            name.to_string()
        };

        let is_pending = date_str.eq_ignore_ascii_case("reserverat");
        let date = if is_pending {
            None
        } else {
            let d = NaiveDate::parse_from_str(date_str, "%Y/%m/%d")
                .map_err(|e| format!("Nordea line {line_no}: bad date '{date_str}': {e}"))?;
            Some(d)
        };

        let amount = parse_swedish_decimal(amount_str)
            .map_err(|e| format!("Nordea line {line_no}: bad amount '{amount_str}': {e}"))?;

        let currency = if currency.is_empty() {
            "SEK".to_string()
        } else {
            currency
        };

        rows.push(ParsedRow {
            date,
            description,
            amount,
            currency,
            is_pending,
        });
    }

    Ok(rows)
}
