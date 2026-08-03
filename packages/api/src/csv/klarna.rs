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
///
/// Some Klarna PDFs are generated with an incomplete ToUnicode CMap (missing glyph→Unicode
/// mappings). In that case `pdf-extract` returns garbled text and zero rows. We detect this and
/// fall back to `pdftotext -layout` (poppler), which recovers the layout but still substitutes
/// two characters due to the same missing CMap: `-` → `j` and `9` → `>`. We normalise those
/// before parsing, which correctly recovers dates and amounts (descriptions stay garbled but are
/// still useful for dedup via SHA-256).
#[cfg(feature = "server")]
pub fn parse(pdf_bytes: &[u8]) -> Result<Vec<ParsedRow>, String> {
    let text = pdf_extract::extract_text_from_mem(pdf_bytes)
        .map_err(|e| format!("Failed to extract text from Klarna PDF: {e}"))?;

    let rows = parse_text(&text);

    // If pdf-extract produced unusable output (garbled font / no line breaks), fall back to
    // pdftotext which handles layout recovery better.
    let rows = match rows {
        Ok(r) if !r.is_empty() => return Ok(r),
        _ => extract_via_pdftotext(pdf_bytes)?,
    };

    Ok(rows)
}

/// Write `pdf_bytes` to a temp file, run `pdftotext -layout`, normalise the two known glyph
/// substitutions (`j`→`-`, `>`→`9`), then parse the recovered text.
#[cfg(feature = "server")]
fn extract_via_pdftotext(pdf_bytes: &[u8]) -> Result<Vec<ParsedRow>, String> {
    use std::io::Write as _;

    // Write bytes to a named temp file that pdftotext can read.
    let tmp_path = std::env::temp_dir().join("klarna_import.pdf");
    let mut f = std::fs::File::create(&tmp_path)
        .map_err(|e| format!("Failed to create temp file for pdftotext: {e}"))?;
    f.write_all(pdf_bytes)
        .map_err(|e| format!("Failed to write temp PDF: {e}"))?;
    drop(f);

    let output = std::process::Command::new("pdftotext")
        .args(["-layout", tmp_path.to_str().unwrap_or(""), "-"])
        .output()
        .map_err(|e| format!("pdftotext not available: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("pdftotext failed: {stderr}"));
    }

    let raw = String::from_utf8_lossy(&output.stdout);

    // Normalise the two glyph substitutions produced by the broken CMap:
    //   '>'  was mapped from '9'  (digit: 2> → 29, >6 → 96, etc.)
    //   'j'  was mapped from '-'  (date separator: 2026j07j28 → 2026-07-28)
    //
    // Descriptions are already garbled by the same CMap issue and are only used for
    // dedup hashing, so a global replacement is acceptable.
    let normalised = raw.replace('>', "9").replace('j', "-");

    parse_text(&normalised)
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
    // We normalise runs of whitespace to a single space so the check works whether the text came
    // from pdf-extract (single space) or pdftotext -layout (column-aligned, multi-space).
    if !text.lines().any(|l| normalise_spaces(l) == HEADER) {
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
        if normalise_spaces(line) == HEADER {
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

/// Collapse all runs of whitespace to a single space and trim the result.
/// Used to match the column header regardless of whether the text came from
/// pdf-extract (single spaces) or pdftotext -layout (wide column gaps).
fn normalise_spaces(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
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

    /// Simulate the pdftotext -layout output format: header and transaction lines have
    /// wide column-aligned spacing.  This exercises the normalise_spaces header detection
    /// and the multi-space-tolerant transaction parser.
    #[test]
    fn test_pdftotext_layout_format() {
        let text = concat!(
            "DATE                    DESCRIPTION                        PAYMENT METHOD     AMOUNT\n",
            "\n",
            "2026-07-11                 U     Uber                      Klarna card      144,00 kr\n",
            "2026-07-04                 N     Naturkompaniet             Klarna card    1 399,00 kr\n",
            "Summary\n",
        );
        let rows = parse_text(text).expect("parse should succeed");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].description, "Uber");
        assert_eq!(rows[0].date, NaiveDate::from_ymd_opt(2026, 7, 11));
        assert_eq!(rows[0].amount.to_string(), "-144.00");
        assert_eq!(rows[1].description, "Naturkompaniet");
        assert_eq!(rows[1].amount.to_string(), "-1399.00");
    }

    /// Verify that the broken-CMap substitutions (j→-, >→9) are correctly reversed so
    /// that dates and amounts parse cleanly via the pdftotext fallback path.
    #[test]
    fn test_glyph_substitution_normalisation() {
        // Raw pdftotext output with substituted glyphs
        let raw = concat!(
            "DATE                    DESCRIPTION          PAYMENT METHOD     AMOUNT\n",
            "2026j07j11                 U     Uber         Klarna card      144,00 kr\n",
            "2026j07j0>                 N     Naturkomp    Klarna card    1 3>>,00 kr\n",
            "Summary\n",
        );
        let normalised = raw.replace('>', "9").replace('j', "-");
        let rows = parse_text(&normalised).expect("parse should succeed");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, NaiveDate::from_ymd_opt(2026, 7, 11));
        assert_eq!(rows[1].date, NaiveDate::from_ymd_opt(2026, 7, 9));
        assert_eq!(rows[1].amount.to_string(), "-1399.00");
    }
}
