//! Naive datetime parser with no added dependencies.
//!
//! Optuna's journal / SQLite store datetimes as strings in the form
//! `YYYY-MM-DDTHH:MM:SS[.ffffff]` or `YYYY-MM-DD HH:MM:SS[.ffffff]` (no timezone
//! information). Since the policy is not to add chrono to the workspace, conversion
//! to unix seconds (f64, naive, no timezone conversion) is done using only the
//! standard library.

/// Converts a naive datetime string to unix seconds (f64).
///
/// Accepted formats:
/// - `YYYY-MM-DDTHH:MM:SS`
/// - `YYYY-MM-DD HH:MM:SS` (separator is 'T' or a space)
/// - An optional trailing fractional-second part `.ffffff` of any number of digits.
///
/// Timezones are not handled; the value as written is returned as-is as a naive
/// epoch second treated as equivalent to UTC. Invalid input returns `None`.
pub fn parse_naive_datetime(s: &str) -> Option<f64> {
    let s = s.trim();
    // Split into the date part (10 chars: YYYY-MM-DD) and the time part at the
    // separator ('T' or ' ').
    let sep = s.as_bytes().get(10).copied()?;
    if sep != b'T' && sep != b' ' {
        return None;
    }
    let (date_part, rest) = s.split_at(10);
    let time_part = &rest[1..]; // skip the single separator character

    let (year, month, day) = parse_date(date_part)?;
    let (hour, minute, second, frac) = parse_time(time_part)?;

    if !(1..=12).contains(&month) || day < 1 || day > days_in_month(year, month) {
        return None;
    }
    if hour > 23 || minute > 59 || second > 60 {
        return None;
    }

    let days = days_from_civil(year, month, day);
    let whole =
        days * 86_400 + i64::from(hour) * 3_600 + i64::from(minute) * 60 + i64::from(second);
    Some(whole as f64 + frac)
}

/// Leap-year check (Gregorian calendar: multiples of 4, excluding multiples of 100,
/// but including multiples of 400).
fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Number of days in the given year/month (assumes month is 1..=12; out-of-range
/// returns 0 and is rejected by the caller's validation).
fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// Splits `YYYY-MM-DD` into (year, month, day).
fn parse_date(s: &str) -> Option<(i64, u32, u32)> {
    let bytes = s.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    let year: i64 = s.get(0..4)?.parse().ok()?;
    let month: u32 = s.get(5..7)?.parse().ok()?;
    let day: u32 = s.get(8..10)?.parse().ok()?;
    Some((year, month, day))
}

/// Splits `HH:MM:SS[.ffffff]` into (hour, minute, second, frac_seconds).
fn parse_time(s: &str) -> Option<(u32, u32, u32, f64)> {
    // Separate out the fractional-second part.
    let (hms, frac) = match s.split_once('.') {
        Some((hms, frac_digits)) => {
            if frac_digits.is_empty() || !frac_digits.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            // Interpreted as "0.<frac_digits>" (any number of digits allowed).
            let frac: f64 = format!("0.{frac_digits}").parse().ok()?;
            (hms, frac)
        }
        None => (s, 0.0),
    };

    let bytes = hms.as_bytes();
    if bytes.len() != 8 || bytes[2] != b':' || bytes[5] != b':' {
        return None;
    }
    let hour: u32 = hms.get(0..2)?.parse().ok()?;
    let minute: u32 = hms.get(3..5)?.parse().ok()?;
    let second: u32 = hms.get(6..8)?.parse().ok()?;
    Some((hour, minute, second, frac))
}

/// days-from-civil algorithm (Howard Hinnant).
/// Returns the epoch day count with 1970-01-01 as 0.
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400; // [0, 399]
    let m = i64::from(month);
    let d = i64::from(day);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146_097 + doe - 719_468
}

/// Converts unix seconds (f64, treated as naive UTC) to "YYYY-MM-DDTHH:MM:SS.ffffff".
/// The inverse of `parse_naive_datetime`. Always uses 6 fixed digits of microseconds.
///
/// Negative unix seconds need not be handled (only 1970 onward is supported), but the
/// function does not panic even if given non-finite values (NaN / Inf) or extreme values.
pub fn format_naive_datetime(unix_secs: f64) -> String {
    // Fall back non-finite values (NaN / Inf) to a safe default (the `as` cast from
    // f64 to i64 is saturating since Rust 1.45 and won't panic, but we still want a
    // meaningful value).
    let unix_secs = if unix_secs.is_finite() {
        unix_secs
    } else {
        0.0
    };
    let total_micros = (unix_secs * 1_000_000.0).round() as i64;

    let secs = total_micros.div_euclid(1_000_000);
    let micros = total_micros.rem_euclid(1_000_000);

    let days = secs.div_euclid(86_400);
    let secs_of_day = secs.rem_euclid(86_400);
    let hour = secs_of_day / 3_600;
    let minute = (secs_of_day % 3_600) / 60;
    let second = secs_of_day % 60;

    let (year, month, day) = civil_from_days(days);

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{micros:06}")
}

/// civil-from-days algorithm (Howard Hinnant). The inverse of `days_from_civil`.
/// Recovers (year, month, day) from the epoch day count with 1970-01-01 as 0.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32; // [1, 12]
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_round_epoch_value() {
        // 2024-01-01T00:00:00 UTC == 1704067200
        assert_eq!(
            parse_naive_datetime("2024-01-01T00:00:00"),
            Some(1_704_067_200.0)
        );
    }

    #[test]
    fn epoch_zero() {
        assert_eq!(parse_naive_datetime("1970-01-01T00:00:00"), Some(0.0));
    }

    #[test]
    fn parses_space_separator() {
        assert_eq!(
            parse_naive_datetime("2024-01-01 00:00:00"),
            Some(1_704_067_200.0)
        );
    }

    #[test]
    fn parses_fractional_seconds() {
        let v = parse_naive_datetime("2024-01-01T00:00:00.500000").unwrap();
        assert!((v - 1_704_067_200.5).abs() < 1e-9);
    }

    #[test]
    fn parses_fractional_seconds_arbitrary_digits() {
        let v = parse_naive_datetime("2024-01-01 00:00:00.123").unwrap();
        assert!((v - 1_704_067_200.123).abs() < 1e-9);
    }

    #[test]
    fn parses_time_of_day() {
        // 2024-01-01T01:02:03 == 1704067200 + 3600 + 120 + 3
        assert_eq!(
            parse_naive_datetime("2024-01-01T01:02:03"),
            Some(1_704_070_923.0)
        );
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(parse_naive_datetime(""), None);
        assert_eq!(parse_naive_datetime("not a date"), None);
        assert_eq!(parse_naive_datetime("2024-01-01"), None);
        assert_eq!(parse_naive_datetime("2024-01-01X00:00:00"), None);
        assert_eq!(parse_naive_datetime("2024/01/01T00:00:00"), None);
        assert_eq!(parse_naive_datetime("2024-13-01T00:00:00"), None);
        assert_eq!(parse_naive_datetime("2024-01-01T25:00:00"), None);
        assert_eq!(parse_naive_datetime("2024-01-01T00:00:00."), None);
        assert_eq!(parse_naive_datetime("2024-01-01T00:00:xx"), None);
    }

    #[test]
    fn rejects_invalid_day_for_month() {
        // A day exceeding the month's day count is invalid (previously a uniform
        // 1..=31 check would have accepted 2/31).
        assert_eq!(parse_naive_datetime("2024-02-31T00:00:00"), None);
        assert_eq!(parse_naive_datetime("2024-04-31T00:00:00"), None);
        assert_eq!(parse_naive_datetime("2024-06-31T00:00:00"), None);
        assert_eq!(parse_naive_datetime("2024-01-32T00:00:00"), None);
        assert_eq!(parse_naive_datetime("2024-01-00T00:00:00"), None);
    }

    #[test]
    fn leap_day_rules() {
        // 2024 is a leap year (multiple of 4) → 2/29 is valid.
        assert!(parse_naive_datetime("2024-02-29T00:00:00").is_some());
        // 2023 is not a leap year → 2/29 is invalid.
        assert_eq!(parse_naive_datetime("2023-02-29T00:00:00"), None);
        // 1900 is a multiple of 100 (but not 400) → not a leap year.
        assert_eq!(parse_naive_datetime("1900-02-29T00:00:00"), None);
        // 2000 is a multiple of 400 → a leap year.
        assert!(parse_naive_datetime("2000-02-29T00:00:00").is_some());
        // 2/28 in a non-leap year is valid.
        assert!(parse_naive_datetime("2023-02-28T00:00:00").is_some());
    }

    #[test]
    fn leap_year_and_later_date() {
        // 2024-02-29T12:00:00 (leap day). Compute via days_from_civil sanity.
        let v = parse_naive_datetime("2024-02-29T12:00:00").unwrap();
        // 2024-02-29 == day 19782 from epoch; *86400 + 43200
        assert_eq!(v, 19_782.0 * 86_400.0 + 43_200.0);
    }

    /// `format_naive_datetime` always outputs 6 fixed digits of microseconds with a "T" separator.
    #[test]
    fn format_naive_datetime_epoch() {
        assert_eq!(format_naive_datetime(0.0), "1970-01-01T00:00:00.000000");
    }

    #[test]
    fn format_naive_datetime_matches_expected_string() {
        let t = parse_naive_datetime("2026-07-16T10:30:00.123456").unwrap();
        assert_eq!(format_naive_datetime(t), "2026-07-16T10:30:00.123456");
    }

    /// Round trip: for representative timestamps (epoch, normal time, leap-year
    /// boundary), `parse_naive_datetime(&format_naive_datetime(t))` matches to
    /// microsecond precision.
    #[test]
    fn datetime_roundtrip_representative_values() {
        let cases = [
            0.0,
            parse_naive_datetime("2026-07-16T10:30:00.123456").unwrap(),
            parse_naive_datetime("2024-02-29T23:59:59.999999").unwrap(),
            parse_naive_datetime("2000-02-29T00:00:00.000000").unwrap(),
            parse_naive_datetime("1970-01-01T00:00:01.000001").unwrap(),
        ];
        for t in cases {
            let formatted = format_naive_datetime(t);
            let roundtripped = parse_naive_datetime(&formatted).unwrap();
            assert!(
                (roundtripped - t).abs() < 1e-6,
                "roundtrip mismatch: t={t} formatted={formatted} roundtripped={roundtripped}"
            );
        }
    }

    /// Passing a non-finite value (NaN / Inf) must not panic.
    #[test]
    fn format_naive_datetime_handles_non_finite_without_panic() {
        let _ = format_naive_datetime(f64::NAN);
        let _ = format_naive_datetime(f64::INFINITY);
        let _ = format_naive_datetime(f64::NEG_INFINITY);
    }
}
