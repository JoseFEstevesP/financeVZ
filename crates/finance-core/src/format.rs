use crate::money::Currency;

const USD_PREFIX: &str = "USD ";
const VES_PREFIX: &str = "Bs.S ";

fn group_digits(digits: &str, separator: char) -> String {
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.bytes().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(separator);
        }
        out.push(digit as char);
    }
    out
}

fn format_parts(cents: i64, prefix: &str, decimal: char, thousands: char) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let absolute = cents.unsigned_abs();
    let whole = group_digits(&(absolute / 100).to_string(), thousands);
    format!("{sign}{prefix}{whole}{decimal}{:02}", absolute % 100)
}

pub fn format_money(cents: i64, currency: &Currency) -> String {
    match currency.as_str() {
        "USD" => format_parts(cents, USD_PREFIX, '.', ','),
        "VES" => format_parts(cents, VES_PREFIX, ',', '.'),
        other => format_parts(cents, &format!("{other} "), '.', ','),
    }
}

pub fn format_rate(rate_cents: i64) -> String {
    format_parts(rate_cents, VES_PREFIX, ',', '.')
}

pub fn usd_to_ves(usd_cents: i64, rate_cents: i64) -> Option<i64> {
    if rate_cents <= 0 {
        return None;
    }
    let value = (usd_cents as i128 * rate_cents as i128 + 50) / 100;
    i64::try_from(value).ok()
}

pub fn ves_to_usd(ves_cents: i64, rate_cents: i64) -> Option<i64> {
    if rate_cents <= 0 {
        return None;
    }
    let value = (ves_cents as i128 * 100 + rate_cents as i128 / 2) / rate_cents as i128;
    i64::try_from(value).ok()
}

/// Máscara del POS: el texto entrante son dígitos de centavos.
pub fn format_cents_input(digits: &str, currency_code: &str) -> String {
    if digits.is_empty() {
        return String::new();
    }
    let padding = 3usize.saturating_sub(digits.len());
    let padded = if padding == 0 {
        digits.to_owned()
    } else {
        format!("{}{}", "0".repeat(padding), digits)
    };
    let split = padded.len() - 2;
    let int_part = &padded[..split];
    let dec_part = &padded[split..];
    let whole: u64 = int_part.parse().unwrap_or(0);
    let (thousands, decimal) = if currency_code == "VES" {
        ('.', ',')
    } else {
        (',', '.')
    };
    format!(
        "{}{}{}",
        group_digits(&whole.to_string(), thousands),
        decimal,
        dec_part
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_each_currency_with_its_own_convention() {
        assert_eq!(
            format_money(123456, &Currency::new("USD").unwrap()),
            "USD 1,234.56"
        );
        assert_eq!(
            format_money(123456, &Currency::new("VES").unwrap()),
            "Bs.S 1.234,56"
        );
        assert_eq!(
            format_money(-5, &Currency::new("USD").unwrap()),
            "-USD 0.05"
        );
        assert_eq!(format_money(0, &Currency::new("VES").unwrap()), "Bs.S 0,00");
    }

    #[test]
    fn formats_the_exchange_rate() {
        assert_eq!(format_rate(85789), "Bs.S 857,89");
    }

    #[test]
    fn converts_both_ways_rounding_to_cents() {
        assert_eq!(usd_to_ves(1000, 85789), Some(857890));
        assert_eq!(ves_to_usd(857890, 85789), Some(1000));
        assert_eq!(ves_to_usd(1000, 0), None);
    }

    #[test]
    fn masks_cents_like_the_pos_input() {
        assert_eq!(format_cents_input("", "USD"), "");
        assert_eq!(format_cents_input("0", "USD"), "0.00");
        assert_eq!(format_cents_input("5", "USD"), "0.05");
        assert_eq!(format_cents_input("250", "USD"), "2.50");
        assert_eq!(format_cents_input("250", "VES"), "2,50");
        assert_eq!(format_cents_input("100000", "USD"), "1,000.00");
        assert_eq!(format_cents_input("100000", "VES"), "1.000,00");
        assert_eq!(format_cents_input("1234567", "VES"), "12.345,67");
    }
}
