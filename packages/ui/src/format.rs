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
