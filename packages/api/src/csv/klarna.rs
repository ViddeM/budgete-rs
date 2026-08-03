use super::{parse_swedish_decimal, ParsedRow};
use chrono::NaiveDate;

/// Parse a Klarna Monthly invoice PDF using OCR.
///
/// Strategy:
/// 1. Write the PDF bytes to a temp file.
/// 2. Render each page to a PNG with `pdftoppm` (poppler-utils).
/// 3. OCR each PNG with `tesseract` (English + Swedish).
/// 4. Concatenate the OCR text and parse transaction lines.
///
/// This approach is immune to broken ToUnicode CMap entries in the PDF, which
/// cause text-extraction tools (`pdf-extract`, `pdftotext`) to produce garbled
/// dates and amounts.
#[cfg(feature = "server")]
pub fn parse(pdf_bytes: &[u8]) -> Result<Vec<ParsedRow>, String> {
    use std::io::Write as _;

    let tmp_dir = tempfile::tempdir().map_err(|e| format!("Failed to create temp dir: {e}"))?;
    let pdf_path = tmp_dir.path().join("klarna.pdf");
    let page_prefix = tmp_dir.path().join("page");

    std::fs::File::create(&pdf_path)
        .and_then(|mut f| f.write_all(pdf_bytes))
        .map_err(|e| format!("Failed to write temp PDF: {e}"))?;

    // Render PDF pages to PNGs: page-1.png, page-2.png, …
    let pdftoppm = std::process::Command::new("pdftoppm")
        .args([
            "-png",
            "-r",
            "200",
            pdf_path.to_str().unwrap(),
            page_prefix.to_str().unwrap(),
        ])
        .output()
        .map_err(|e| format!("pdftoppm not available: {e}"))?;

    if !pdftoppm.status.success() {
        return Err(format!(
            "pdftoppm failed: {}",
            String::from_utf8_lossy(&pdftoppm.stderr)
        ));
    }

    // Collect rendered page images in order.
    let mut pages: Vec<std::path::PathBuf> = std::fs::read_dir(tmp_dir.path())
        .map_err(|e| format!("Failed to read temp dir: {e}"))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("png"))
        .collect();
    pages.sort();

    // OCR each page and concatenate the text.
    let mut full_text = String::new();
    for page in &pages {
        // tesseract <input> stdout -l eng+swe --psm 6
        let ocr = std::process::Command::new("tesseract")
            .args([
                page.to_str().unwrap(),
                "stdout",
                "-l",
                "eng+swe",
                "--psm",
                "6",
            ])
            .output()
            .map_err(|e| format!("tesseract not available: {e}"))?;

        if !ocr.status.success() {
            return Err(format!(
                "tesseract failed: {}",
                String::from_utf8_lossy(&ocr.stderr)
            ));
        }

        full_text.push_str(&String::from_utf8_lossy(&ocr.stdout));
        full_text.push('\n');
    }

    parse_text(&full_text)
}

/// Parse OCR'd plain text from a Klarna Monthly invoice.
///
/// Each transaction line looks like:
///   `"2026-06-13 U Uber Klarna card 144,00 kr"`
///
/// Strategy:
/// 1. Confirm the text is from a Klarna invoice via the language-independent sentinel.
/// 2. Collect every line that parses as a transaction (starts with YYYY-MM-DD).
/// 3. Stop at the summary footer ("Summary" / "Summering").
fn parse_text(text: &str) -> Result<Vec<ParsedRow>, String> {
    // "Klarna Bank AB" appears on the first page of every invoice regardless of language.
    if !text.contains("Klarna Bank AB") {
        return Err("Could not find Klarna Bank AB header in PDF".to_string());
    }

    let mut rows: Vec<ParsedRow> = Vec::new();

    for line in text.lines() {
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        // Stop at the summary footer (English or Swedish).
        if line == "Summary" || line == "Summering" {
            break;
        }

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
/// - Payment method is one of "Klarna card", "Pay later", "Klarna kortet", "Betala senare".
/// - Everything between the icon and the payment method is the description.
fn parse_transaction_line(line: &str) -> Option<ParsedRow> {
    if line.len() < 10 {
        return None;
    }
    let date = NaiveDate::parse_from_str(&line[..10], "%Y-%m-%d").ok()?;

    let rest = line.get(11..)?.trim();
    let rest = rest.strip_suffix(" kr")?;

    let amount_str = extract_amount_from_end(rest)?;
    let amount_end = rest.len() - amount_str.len();
    let before_amount = rest[..amount_end].trim_end();

    let raw_amount = parse_swedish_decimal(amount_str).ok()?;

    let before_pm = strip_payment_method(before_amount)?;
    let before_pm = before_pm.trim_end();

    let mut chars = before_pm.chars();
    let _icon = chars.next()?;
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

/// Extract the numeric amount string from the right end of a field string.
///
/// Swedish amounts: `"144,00"` or `"1 399,00"` (space as thousands separator).
fn extract_amount_from_end(s: &str) -> Option<&str> {
    let comma_pos = s.rfind(',')?;
    let prefix = &s[..comma_pos];
    let amount_start = if let Some(space_pos) = prefix.rfind(' ') {
        let before_space = &prefix[..space_pos];
        if before_space.ends_with(|c: char| c.is_ascii_digit()) {
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
    for pm in ["Klarna card", "Pay later", "Klarna kortet", "Betala senare"] {
        if let Some(stripped) = s.strip_suffix(pm) {
            return Some(stripped);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Representative lines matching clean OCR output (English invoice).
    const SAMPLE: &str = r#"
Klarna Bank AB (publ), FE 50500

Date sent: 2026-06-15

DATE DESCRIPTION PAYMENT METHOD AMOUNT

2026-06-13 U Uber Klarna card 144,00 kr

2026-06-04 N Naturkompaniet Klarna card 1 399,00 kr

2026-06-02 S Skicka blommor med Blomsterlandet Pay later 536,00 kr

2026-06-04 S Steam Klarna card 109,11 kr

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
    fn test_no_klarna_header_returns_error() {
        let result = parse_text("some random text without the header");
        assert!(result.is_err());
    }

    #[test]
    fn test_summary_stops_parsing() {
        let text = concat!(
            "Klarna Bank AB (publ), FE 50500\n",
            "2026-06-13 U Uber Klarna card 144,00 kr\n",
            "Summary\n",
            "2026-06-14 X Fake Klarna card 999,00 kr\n",
        );
        let rows = parse_text(text).expect("parse should succeed");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].description, "Uber");
    }

    #[test]
    fn test_swedish_invoice() {
        let text = concat!(
            "Klarna Bank AB (publ), FE 50500\n",
            "2026-06-08 C Coop Klarna kortet 53,14 kr\n",
            "2026-06-14 E ETSY IRELAND Betala senare 34,39 kr\n",
            "Summering\n",
        );
        let rows = parse_text(text).expect("parse should succeed");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].description, "Coop");
        assert_eq!(rows[0].amount.to_string(), "-53.14");
        assert_eq!(rows[1].description, "ETSY IRELAND");
        assert_eq!(rows[1].amount.to_string(), "-34.39");
    }

    #[test]
    fn test_thousands_separator_in_amount() {
        let text = concat!(
            "Klarna Bank AB (publ), FE 50500\n",
            "2026-05-25 L Lidl Klarna card 1 335,54 kr\n",
            "Summary\n",
        );
        let rows = parse_text(text).expect("parse should succeed");
        assert_eq!(rows[0].amount.to_string(), "-1335.54");
    }
}
