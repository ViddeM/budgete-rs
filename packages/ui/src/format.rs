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
