//! P5R calendar helpers. Day 0 is April 1 of the game year, which was a Friday.

pub const DAY_MIN: u16 = 0;
pub const DAY_MAX: u16 = 364;

/// Time-of-day values used by the save format, in display order.
pub const TIMES: [(u8, &str); 9] = [
    (0, "朝"),
    (1, "昼"),
    (2, "午後"),
    (3, "日中"),
    (4, "放課後"),
    (5, "夜"),
    (6, "深夜"),
    (8, "ダンジョン後"),
    (9, "深夜2"),
];

pub fn month_days(month: u8) -> Option<u8> {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Some(31),
        4 | 6 | 9 | 11 => Some(30),
        2 => Some(28),
        _ => None,
    }
}

/// Converts a month/day pair (April through March) to the save's day index.
pub fn date_to_day(month: u8, day: u8) -> Option<u16> {
    if day == 0 || day > month_days(month)? {
        return None;
    }
    let mut total = 0u16;
    if month >= 4 {
        for m in 4..month {
            total += month_days(m)? as u16;
        }
        total += (day - 1) as u16;
    } else {
        for m in 4..=12 {
            total += month_days(m)? as u16;
        }
        for m in 1..month {
            total += month_days(m)? as u16;
        }
        total += (day - 1) as u16;
    }
    (total <= DAY_MAX).then_some(total)
}

/// Converts a save day index to a month/day pair.
pub fn day_to_date(day: u16) -> Option<(u8, u8)> {
    if day > DAY_MAX {
        return None;
    }
    let mut remaining = day;
    let months: [u8; 12] = [4, 5, 6, 7, 8, 9, 10, 11, 12, 1, 2, 3];
    for month in months {
        let length = month_days(month)? as u16;
        if remaining < length {
            return Some((month, remaining as u8 + 1));
        }
        remaining -= length;
    }
    None
}

/// Weekday for a day index. April 1 is Friday.
pub fn weekday(day: u16) -> &'static str {
    const NAMES: [&str; 7] = ["金", "土", "日", "月", "火", "水", "木"];
    NAMES[(day % 7) as usize]
}

pub fn format_date(day: u16) -> String {
    match day_to_date(day) {
        Some((month, day_of_month)) => {
            format!("{month}月{day_of_month}日({})", weekday(day))
        }
        None => format!("範囲外({day})"),
    }
}

pub fn format_date_short(day: u16) -> String {
    match day_to_date(day) {
        Some((month, day_of_month)) => format!("{month}/{day_of_month}"),
        None => "?".into(),
    }
}

pub fn time_label(time: u8) -> &'static str {
    TIMES
        .iter()
        .find(|(value, _)| *value == time)
        .map(|(_, label)| *label)
        .unwrap_or("不明")
}

/// The shorter label written into the save-select description.
pub fn time_desc_label(time: u8) -> &'static str {
    match time {
        0 => "朝",
        1 => "昼",
        2 => "午後",
        3 => "午後",
        4 => "放課後",
        5 => "夜",
        6 => "深夜",
        8 => "夜",
        9 => "深夜",
        _ => "不明",
    }
}

pub fn is_valid_time(time: u8) -> bool {
    TIMES.iter().any(|(value, _)| *value == time)
}

/// Parses `12/24`, `12月24日`, `12-24` and `12 24`, normalising full-width input.
pub fn parse_date(text: &str) -> Option<u16> {
    let normalized = crate::input::normalize_text(text);
    let normalized = normalized.trim();
    if normalized.is_empty() {
        return None;
    }
    let replaced = normalized
        .replace('月', "/")
        .replace('日', "")
        .replace(['-', '.', ' '], "/");
    let mut parts = replaced.split('/').filter(|part| !part.is_empty());
    let month: u8 = parts.next()?.parse().ok()?;
    let day: u8 = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    date_to_day(month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_dates() {
        assert_eq!(date_to_day(4, 1), Some(0));
        assert_eq!(date_to_day(12, 24), Some(267));
        assert_eq!(date_to_day(3, 20), Some(353));
        assert_eq!(date_to_day(3, 31), Some(364));
        assert_eq!(day_to_date(0), Some((4, 1)));
        assert_eq!(day_to_date(267), Some((12, 24)));
        assert_eq!(day_to_date(353), Some((3, 20)));
        assert_eq!(day_to_date(364), Some((3, 31)));
    }

    #[test]
    fn round_trips_every_day() {
        for day in DAY_MIN..=DAY_MAX {
            let (month, day_of_month) = day_to_date(day).expect("in range");
            assert_eq!(date_to_day(month, day_of_month), Some(day), "day {day}");
        }
    }

    #[test]
    fn weekdays_match_known_saves() {
        assert_eq!(weekday(267), "土");
        assert_eq!(weekday(353), "月");
        assert_eq!(weekday(0), "金");
    }

    #[test]
    fn rejects_invalid_dates() {
        assert_eq!(date_to_day(2, 29), None);
        assert_eq!(date_to_day(4, 31), None);
        assert_eq!(date_to_day(0, 1), None);
        assert_eq!(date_to_day(13, 1), None);
    }

    #[test]
    fn parses_variants() {
        assert_eq!(parse_date("12/24"), Some(267));
        assert_eq!(parse_date("１２月２４日"), Some(267));
        assert_eq!(parse_date("3-20"), Some(353));
        assert_eq!(parse_date(" 4 1 "), Some(0));
        assert_eq!(parse_date("2/29"), None);
        assert_eq!(parse_date(""), None);
    }

    #[test]
    fn time_labels_are_stable() {
        assert_eq!(time_label(0), "朝");
        assert_eq!(time_label(4), "放課後");
        assert_eq!(time_label(5), "夜");
        assert_eq!(time_label(6), "深夜");
        assert_eq!(time_label(3), "日中");
        assert_eq!(time_desc_label(3), "午後");
        assert_eq!(time_desc_label(8), "夜");
        assert_eq!(time_desc_label(9), "深夜");
        assert!(!is_valid_time(7));
    }
}
