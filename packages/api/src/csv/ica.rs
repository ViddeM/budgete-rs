use super::{parse_swedish_decimal, ParsedRow};
use chrono::NaiveDate;

/// Parse an ICA Bank CSV export.
///
/// Format: `Datum;Text;Typ;Belopp;Saldo`
/// - Separator: `;`
/// - Date: `YYYY-MM-DD`
/// - Amount (`Belopp`): Swedish decimal with ` kr` suffix.
///   Negative = expense, positive = income — already correct, import as-is.
/// - The `Saldo` (balance) column is ignored.
/// - Currency is always SEK; there are no pending rows.
pub fn parse(content: &str) -> Result<Vec<ParsedRow>, String> {
    let mut rows = Vec::new();

    for (line_no, line) in content.lines().enumerate() {
        // Skip the header row.
        if line_no == 0 {
            continue;
        }

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Fields: Datum ; Text ; Typ ; Belopp ; Saldo
        let fields: Vec<&str> = line.splitn(5, ';').collect();
        if fields.len() < 4 {
            return Err(format!(
                "ICA line {line_no}: expected at least 4 fields, got {}",
                fields.len()
            ));
        }

        let date_str = fields[0].trim();
        let description = fields[1].trim().to_string();
        let amount_str = fields[3].trim();

        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .map_err(|e| format!("ICA line {line_no}: bad date '{date_str}': {e}"))?;

        // Amount has a " kr" suffix, e.g. "-35,00 kr" or "1 500,00 kr".
        let amount_num = amount_str.strip_suffix(" kr").ok_or_else(|| {
            format!("ICA line {line_no}: expected ' kr' suffix in '{amount_str}'")
        })?;

        let amount = parse_swedish_decimal(amount_num)
            .map_err(|e| format!("ICA line {line_no}: bad amount '{amount_num}': {e}"))?;

        rows.push(ParsedRow {
            date: Some(date),
            description,
            // ICA amounts are already signed correctly (negative = expense).
            amount,
            currency: "SEK".to_string(),
            is_pending: false,
        });
    }

    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
Datum;Text;Typ;Belopp;Saldo
2026-06-18;Avgift Bankkort;Övrigt;-35,00 kr;634,10 kr
2026-05-25;Överföring;Insättning;1 500,00 kr;3 511,10 kr
2026-05-12;Ryde Sweden Ab                ;Korttransaktion;-7,40 kr;2 015,10 kr
2026-05-06;Sveriges Lärare(Medlemsavgifter);PG/BG-betalning;-584,00 kr;2 022,50 kr
";

    #[test]
    fn test_parse_sample() {
        let rows = parse(SAMPLE).expect("parse should succeed");
        assert_eq!(rows.len(), 4);

        // Expense: negative sign preserved as-is.
        assert_eq!(rows[0].description, "Avgift Bankkort");
        assert_eq!(rows[0].date, NaiveDate::from_ymd_opt(2026, 6, 18));
        assert_eq!(rows[0].amount.to_string(), "-35.00");

        // Income: positive sign preserved as-is.
        assert_eq!(rows[1].description, "Överföring");
        assert_eq!(rows[1].amount.to_string(), "1500.00");

        // Description is trimmed of trailing whitespace.
        assert_eq!(rows[2].description, "Ryde Sweden Ab");
        assert_eq!(rows[2].amount.to_string(), "-7.40");

        // Description with parentheses (contains semicolon-safe content).
        assert_eq!(rows[3].description, "Sveriges Lärare(Medlemsavgifter)");
        assert_eq!(rows[3].amount.to_string(), "-584.00");

        for row in &rows {
            assert_eq!(row.currency, "SEK");
            assert!(!row.is_pending);
        }
    }

    #[test]
    fn test_empty_after_header() {
        let rows = parse("Datum;Text;Typ;Belopp;Saldo\n").expect("parse should succeed");
        assert!(rows.is_empty());
    }
}
