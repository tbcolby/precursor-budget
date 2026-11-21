//! Money as integer minor units (cents). No floating point anywhere.

/// Largest amount one transaction may hold: 999,999,999.99.
pub const MAX_AMOUNT: u64 = 99_999_999_999;
/// Longest amount string the form accepts (9 integer digits, '.', 2 decimals).
pub const MAX_AMOUNT_CHARS: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmountError {
    Empty,
    /// Not digits with at most one '.' and at most two decimals.
    Invalid,
    TooLarge,
}

impl AmountError {
    pub fn message(self) -> &'static str {
        match self {
            AmountError::Empty => "Enter an amount",
            AmountError::Invalid => "Amount must look like 12.34",
            AmountError::TooLarge => "Amount too large (max 999,999,999.99)",
        }
    }
}

/// Parse `"12"`, `"12.5"`, `"12.50"`, `".5"` or `"12."` into minor units.
/// Rejects more than two decimals, anything but digits and one '.', and
/// values above [`MAX_AMOUNT`]. Overflow is impossible: every step is checked.
pub fn parse_amount(s: &str) -> Result<u64, AmountError> {
    if s.is_empty() {
        return Err(AmountError::Empty);
    }
    let (int, frac) = match s.split_once('.') {
        Some((i, f)) => (i, f),
        None => (s, ""),
    };
    if !int.bytes().all(|b| b.is_ascii_digit()) || !frac.bytes().all(|b| b.is_ascii_digit()) || frac.len() > 2 {
        return Err(AmountError::Invalid);
    }
    let mut units: u64 = 0;
    for b in int.bytes() {
        units = units
            .checked_mul(10)
            .and_then(|u| u.checked_add((b - b'0') as u64))
            .ok_or(AmountError::TooLarge)?;
        if units > MAX_AMOUNT {
            return Err(AmountError::TooLarge);
        }
    }
    let mut cents: u64 = 0;
    for (i, b) in frac.bytes().enumerate() {
        cents += (b - b'0') as u64 * if i == 0 { 10 } else { 1 };
    }
    let total = units.checked_mul(100).and_then(|u| u.checked_add(cents)).ok_or(AmountError::TooLarge)?;
    if total > MAX_AMOUNT {
        return Err(AmountError::TooLarge);
    }
    Ok(total)
}

/// Whether `s` is acceptable while the user is still typing: empty, or
/// something [`parse_amount`] accepts, within [`MAX_AMOUNT_CHARS`].
pub fn amount_input_ok(s: &str) -> bool { s.len() <= MAX_AMOUNT_CHARS && (s.is_empty() || parse_amount(s).is_ok()) }

/// The editable text for an amount (`1250` -> `"12.50"`).
pub fn amount_to_input(minor: u64) -> String { format!("{}.{:02}", minor / 100, minor % 100) }

/// Format signed minor units with a currency symbol and thousands
/// separators: `-123456` with `"$"` -> `"-$1,234.56"`.
pub fn format_amount(minor: i64, symbol: &str) -> String {
    let sign = if minor < 0 { "-" } else { "" };
    let abs = minor.unsigned_abs();
    let whole = (abs / 100).to_string();
    let mut grouped = String::with_capacity(whole.len() + whole.len() / 3);
    for (i, c) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(c);
    }
    format!("{sign}{symbol}{grouped}.{:02}", abs % 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_forms() {
        assert_eq!(parse_amount("12"), Ok(1200));
        assert_eq!(parse_amount("12.5"), Ok(1250));
        assert_eq!(parse_amount("12.50"), Ok(1250));
        assert_eq!(parse_amount("12.05"), Ok(1205));
        assert_eq!(parse_amount(".5"), Ok(50));
        assert_eq!(parse_amount("12."), Ok(1200));
        assert_eq!(parse_amount("0"), Ok(0));
        assert_eq!(parse_amount("."), Ok(0));
        assert_eq!(parse_amount("007.10"), Ok(710));
    }

    #[test]
    fn rejects_malformed() {
        assert_eq!(parse_amount(""), Err(AmountError::Empty));
        assert_eq!(parse_amount("1.234"), Err(AmountError::Invalid));
        assert_eq!(parse_amount("1.2.3"), Err(AmountError::Invalid));
        assert_eq!(parse_amount("-5"), Err(AmountError::Invalid));
        assert_eq!(parse_amount("1,000"), Err(AmountError::Invalid));
        assert_eq!(parse_amount("1e3"), Err(AmountError::Invalid));
    }

    /// Audit regression: eight digits wrapped a u32 (50000000 -> 705,032,704
    /// cents) and ten digits silently became 0.
    #[test]
    fn large_amounts_never_wrap() {
        assert_eq!(parse_amount("50000000"), Ok(5_000_000_000));
        assert_eq!(parse_amount("999999999.99"), Ok(MAX_AMOUNT));
        assert_eq!(parse_amount("1000000000"), Err(AmountError::TooLarge));
        assert_eq!(parse_amount("9999999999"), Err(AmountError::TooLarge));
        assert_eq!(parse_amount("99999999999999999999999"), Err(AmountError::TooLarge));
    }

    #[test]
    fn input_filter() {
        assert!(amount_input_ok(""));
        assert!(amount_input_ok("12."));
        assert!(amount_input_ok("12.3"));
        assert!(!amount_input_ok("12.345"));
        assert!(!amount_input_ok("12a"));
        assert!(!amount_input_ok("1000000000"));
        assert!(amount_input_ok("999999999.99"));
    }

    #[test]
    fn formats_with_grouping() {
        assert_eq!(format_amount(0, "$"), "$0.00");
        assert_eq!(format_amount(5, "$"), "$0.05");
        assert_eq!(format_amount(123_456, "$"), "$1,234.56");
        assert_eq!(format_amount(-123_456, "€"), "-€1,234.56");
        assert_eq!(format_amount(100_000_000, ""), "1,000,000.00");
        assert_eq!(format_amount(99_999, "£"), "£999.99");
        assert_eq!(format_amount(i64::MIN, "$"), "-$92,233,720,368,547,758.08");
    }

    #[test]
    fn input_round_trip() {
        for v in [0, 1, 99, 100, 1250, MAX_AMOUNT] {
            assert_eq!(parse_amount(&amount_to_input(v)), Ok(v));
        }
    }
}
