use chrono::NaiveDate;
use rust_decimal::{prelude::ToPrimitive, Decimal};

/// Insert a narrow no-break space (U+202F) every three digits from the right
/// into the digit string `s` (no sign, no decimal point).
fn thousands(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push('\u{202F}');
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}

/// Format a `Decimal` as a rounded integer with Swedish thousands separators
/// and a ` SEK` suffix.  Negative values are preserved.
///
/// Examples: `12345.67` → `"12 345 SEK"`, `-1234` → `"-1 234 SEK"`.
pub fn fmt_amount(d: Decimal) -> String {
    let n = d.round().to_i64().unwrap_or(0);
    let negative = n < 0;
    let formatted = thousands(&n.unsigned_abs().to_string());
    if negative {
        format!("-{formatted} SEK")
    } else {
        format!("{formatted} SEK")
    }
}

/// Format a transaction amount with two decimal places, Swedish thousands
/// separators on the integer part, and the transaction's own currency code.
///
/// Examples: `1234.5` + `"SEK"` → `"1 234.50 SEK"`,
///           `-999.0` + `"EUR"` → `"-999.00 EUR"`.
pub fn fmt_tx_amount(d: Decimal, currency: &str) -> String {
    let f = d.to_f64().unwrap_or(0.0);
    let negative = f < 0.0;
    let abs_f = f.abs();
    let int_part = abs_f as u64;
    // Two decimal digits from the fractional part, rounded.
    let frac = ((abs_f - int_part as f64) * 100.0).round() as u32;
    let formatted_int = thousands(&int_part.to_string());
    let sign = if negative { "-" } else { "" };
    format!("{sign}{formatted_int}.{frac:02} {currency}")
}

/// Parse perceived luminance (0–255) from a CSS hex color string (`#rrggbb`).
/// Returns `None` if the string can't be parsed.
fn perceived_luminance(hex: &str) -> Option<f32> {
    let hex = hex.trim_start_matches('#');
    if hex.len() < 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f32;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f32;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f32;
    // Standard perceptual luminance coefficients (Rec. 601).
    Some(0.299 * r + 0.587 * g + 0.114 * b)
}

/// Returns a CSS color value suitable for text rendered on top of `hex`.
/// Dark text for bright backgrounds, white for dark ones.
pub fn contrast_text(hex: &str) -> &'static str {
    match perceived_luminance(hex) {
        Some(l) if l > 155.0 => "#111827",
        _ => "#ffffff",
    }
}

/// Returns a CSS `filter` value for a subtle hover highlight that looks right
/// regardless of whether the background is light or dark.
pub fn hover_filter(hex: &str) -> &'static str {
    match perceived_luminance(hex) {
        Some(l) if l > 155.0 => "brightness(0.88)",
        _ => "brightness(1.18)",
    }
}

/// Format an optional transaction date as `"YYYY-MM-DD"`, or `"Pending"` when
/// the date is `None` (i.e. the transaction is a Nordea *Reserverat* row).
pub fn fmt_date(date: Option<NaiveDate>) -> String {
    date.map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "Pending".to_string())
}

/// Returns the CSS color used to render a transaction amount: green for
/// income (≥ 0), red for expenses (< 0).
pub fn tx_amount_color(amount: Decimal) -> &'static str {
    if amount >= Decimal::ZERO {
        "#16a34a"
    } else {
        "#dc2626"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    // U+202F NARROW NO-BREAK SPACE used as the thousands separator.
    const NBSP: char = '\u{202F}';

    fn dec(s: &str) -> Decimal {
        s.parse().unwrap()
    }

    // ── fmt_amount ──────────────────────────────────────────────────────────

    #[test]
    fn fmt_amount_positive_with_thousands() {
        assert_eq!(fmt_amount(dec("12345.67")), format!("12{NBSP}346 SEK"));
    }

    #[test]
    fn fmt_amount_negative_with_thousands() {
        assert_eq!(fmt_amount(dec("-1234.00")), format!("-1{NBSP}234 SEK"));
    }

    #[test]
    fn fmt_amount_zero() {
        assert_eq!(fmt_amount(Decimal::ZERO), "0 SEK");
    }

    #[test]
    fn fmt_amount_small_no_separator() {
        assert_eq!(fmt_amount(dec("42.49")), "42 SEK");
    }

    // ── fmt_tx_amount ───────────────────────────────────────────────────────

    #[test]
    fn fmt_tx_amount_positive_with_thousands() {
        assert_eq!(
            fmt_tx_amount(dec("1234.50"), "SEK"),
            format!("1{NBSP}234.50 SEK")
        );
    }

    #[test]
    fn fmt_tx_amount_negative_no_thousands() {
        assert_eq!(fmt_tx_amount(dec("-999.00"), "EUR"), "-999.00 EUR");
    }

    #[test]
    fn fmt_tx_amount_zero_pads_decimals() {
        assert_eq!(fmt_tx_amount(Decimal::ZERO, "SEK"), "0.00 SEK");
    }

    #[test]
    fn fmt_tx_amount_different_currency() {
        assert_eq!(fmt_tx_amount(dec("100.99"), "USD"), "100.99 USD");
    }

    // ── contrast_text ───────────────────────────────────────────────────────

    #[test]
    fn contrast_text_bright_color_returns_dark_text() {
        // Pure white: luminance ≈ 255, well above threshold 155.
        assert_eq!(contrast_text("#ffffff"), "#111827");
    }

    #[test]
    fn contrast_text_dark_color_returns_white_text() {
        // Pure black: luminance = 0, below threshold 155.
        assert_eq!(contrast_text("#000000"), "#ffffff");
    }

    #[test]
    fn contrast_text_medium_bright_yellow_returns_dark() {
        // Yellow #ffff00: luminance = 0.299*255 + 0.587*255 ≈ 226, above 155.
        assert_eq!(contrast_text("#ffff00"), "#111827");
    }

    #[test]
    fn contrast_text_invalid_hex_returns_white() {
        // perceived_luminance returns None → falls through to "#ffffff".
        assert_eq!(contrast_text("not-a-color"), "#ffffff");
    }

    // ── fmt_date ────────────────────────────────────────────────────────────

    #[test]
    fn fmt_date_some_formats_iso() {
        let d = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        assert_eq!(fmt_date(Some(d)), "2026-01-15");
    }

    #[test]
    fn fmt_date_none_returns_pending() {
        assert_eq!(fmt_date(None), "Pending");
    }

    // ── tx_amount_color ─────────────────────────────────────────────────────

    #[test]
    fn tx_amount_color_negative_is_red() {
        assert_eq!(tx_amount_color(dec("-100.00")), "#dc2626");
    }

    #[test]
    fn tx_amount_color_zero_is_green() {
        assert_eq!(tx_amount_color(Decimal::ZERO), "#16a34a");
    }

    #[test]
    fn tx_amount_color_positive_is_green() {
        assert_eq!(tx_amount_color(dec("50.00")), "#16a34a");
    }

    // ── hover_filter ─────────────────────────────────────────────────────────

    #[test]
    fn hover_filter_dark_background_brightens() {
        assert_eq!(hover_filter("#000000"), "brightness(1.18)");
    }

    #[test]
    fn hover_filter_light_background_darkens() {
        assert_eq!(hover_filter("#ffffff"), "brightness(0.88)");
    }
}
