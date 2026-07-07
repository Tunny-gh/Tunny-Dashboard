//! 依存追加なしの naive 日時パーサ。
//!
//! Optuna の journal / SQLite は日時を `YYYY-MM-DDTHH:MM:SS[.ffffff]` または
//! `YYYY-MM-DD HH:MM:SS[.ffffff]` の文字列で持つ（タイムゾーン情報なし）。
//! chrono をワークスペースに追加しない方針のため、標準ライブラリのみで
//! unix 秒（f64、naive・タイムゾーン変換なし）へ変換する。

/// naive 日時文字列を unix 秒（f64）へ変換する。
///
/// 受け付ける形式:
/// - `YYYY-MM-DDTHH:MM:SS`
/// - `YYYY-MM-DD HH:MM:SS`（区切りは 'T' またはスペース）
/// - 末尾に任意桁の小数秒 `.ffffff` を許容する。
///
/// タイムゾーンは扱わず、記載値をそのまま UTC 相当の naive epoch 秒として返す。
/// 不正な入力は `None` を返す。
pub fn parse_naive_datetime(s: &str) -> Option<f64> {
    let s = s.trim();
    // 日付部（10 文字: YYYY-MM-DD）と時刻部を区切り文字（'T' or ' '）で分割する。
    let sep = s.as_bytes().get(10).copied()?;
    if sep != b'T' && sep != b' ' {
        return None;
    }
    let (date_part, rest) = s.split_at(10);
    let time_part = &rest[1..]; // 区切り 1 文字をスキップ

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

/// 閏年判定（グレゴリオ暦: 4 の倍数、ただし 100 の倍数は除き 400 の倍数は含む）。
fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// 指定年月の日数（month は 1..=12 前提。範囲外は 0 を返し呼び出し側の検証で弾かれる）。
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

/// `YYYY-MM-DD` を (year, month, day) へ分解する。
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

/// `HH:MM:SS[.ffffff]` を (hour, minute, second, frac_seconds) へ分解する。
fn parse_time(s: &str) -> Option<(u32, u32, u32, f64)> {
    // 小数秒を分離する。
    let (hms, frac) = match s.split_once('.') {
        Some((hms, frac_digits)) => {
            if frac_digits.is_empty() || !frac_digits.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            // "0.<frac_digits>" として解釈する（桁数は任意）。
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

/// days-from-civil アルゴリズム（Howard Hinnant）。
/// 1970-01-01 を 0 とする epoch 日数を返す。
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
        // 月別日数を超える日は不正（従来は 1..=31 の一律チェックで 2/31 を受理していた）。
        assert_eq!(parse_naive_datetime("2024-02-31T00:00:00"), None);
        assert_eq!(parse_naive_datetime("2024-04-31T00:00:00"), None);
        assert_eq!(parse_naive_datetime("2024-06-31T00:00:00"), None);
        assert_eq!(parse_naive_datetime("2024-01-32T00:00:00"), None);
        assert_eq!(parse_naive_datetime("2024-01-00T00:00:00"), None);
    }

    #[test]
    fn leap_day_rules() {
        // 2024 は閏年（4 の倍数）→ 2/29 は有効。
        assert!(parse_naive_datetime("2024-02-29T00:00:00").is_some());
        // 2023 は平年 → 2/29 は不正。
        assert_eq!(parse_naive_datetime("2023-02-29T00:00:00"), None);
        // 1900 は 100 の倍数（400 の倍数でない）→ 平年。
        assert_eq!(parse_naive_datetime("1900-02-29T00:00:00"), None);
        // 2000 は 400 の倍数 → 閏年。
        assert!(parse_naive_datetime("2000-02-29T00:00:00").is_some());
        // 平年の 2/28 は有効。
        assert!(parse_naive_datetime("2023-02-28T00:00:00").is_some());
    }

    #[test]
    fn leap_year_and_later_date() {
        // 2024-02-29T12:00:00 (leap day). Compute via days_from_civil sanity.
        let v = parse_naive_datetime("2024-02-29T12:00:00").unwrap();
        // 2024-02-29 == day 19782 from epoch; *86400 + 43200
        assert_eq!(v, 19_782.0 * 86_400.0 + 43_200.0);
    }
}
