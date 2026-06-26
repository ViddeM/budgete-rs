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

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
Bokföringsdag;Belopp;Avsändare;Mottagare;Namn;Rubrik;Saldo;Valuta;
2026/01/15;-112,50;;;ICA FOCUS;Mat och livsmedel;45 000,00;SEK;
2026/01/10;5 000,00;;;Arbetsgivaren;Lön januari;50 000,00;SEK;
Reserverat;-75,00;;;Klarna;Onlineköp;44 000,00;SEK;
";

    #[test]
    fn test_parse_sample() {
        let rows = parse(SAMPLE).expect("parse should succeed");
        assert_eq!(rows.len(), 3);

        // Settled expense: negative amount preserved, Rubrik used as description.
        assert_eq!(rows[0].description, "Mat och livsmedel");
        assert_eq!(rows[0].date, NaiveDate::from_ymd_opt(2026, 1, 15));
        assert_eq!(rows[0].amount.to_string(), "-112.50");
        assert_eq!(rows[0].currency, "SEK");
        assert!(!rows[0].is_pending);

        // Settled income: positive amount.
        assert_eq!(rows[1].description, "Lön januari");
        assert_eq!(rows[1].amount.to_string(), "5000.00");
        assert!(!rows[1].is_pending);

        // Pending row: date is None, is_pending is true.
        assert_eq!(rows[2].description, "Onlineköp");
        assert!(rows[2].date.is_none());
        assert!(rows[2].is_pending);
        assert_eq!(rows[2].amount.to_string(), "-75.00");
    }

    #[test]
    fn test_empty_rubrik_falls_back_to_namn() {
        let csv = "Bokföringsdag;Belopp;Avsändare;Mottagare;Namn;Rubrik;Saldo;Valuta;\n\
                   2026/01/15;-112,50;;;ICA FOCUS;;45 000,00;SEK;\n";
        let rows = parse(csv).expect("parse should succeed");
        assert_eq!(rows[0].description, "ICA FOCUS");
    }

    #[test]
    fn test_empty_currency_defaults_to_sek() {
        let csv = "Bokföringsdag;Belopp;Avsändare;Mottagare;Namn;Rubrik;Saldo;Valuta;\n\
                   2026/01/15;-112,50;;;ICA;Mat;45000;;\n";
        let rows = parse(csv).expect("parse should succeed");
        assert_eq!(rows[0].currency, "SEK");
    }

    #[test]
    fn test_bom_at_start_is_ignored() {
        // UTF-8 BOM prepended to the first (header) line.
        let csv = "\u{feff}Bokföringsdag;Belopp;Avsändare;Mottagare;Namn;Rubrik;Saldo;Valuta;\n\
                   2026/01/15;-50,00;;;Coop;Dagligvaror;10 000,00;SEK;\n";
        let rows = parse(csv).expect("parse with BOM should succeed");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].description, "Dagligvaror");
    }

    #[test]
    fn test_empty_after_header() {
        let csv = "Bokföringsdag;Belopp;Avsändare;Mottagare;Namn;Rubrik;Saldo;Valuta;\n";
        let rows = parse(csv).expect("parse should succeed");
        assert!(rows.is_empty());
    }

    #[test]
    fn test_too_few_fields_returns_error() {
        let csv = "Bokföringsdag;Belopp;Avsändare;Mottagare;Namn;Rubrik;\n\
                   2026/01/15;-50,00;;;Coop;\n";
        assert!(parse(csv).is_err());
    }
}
