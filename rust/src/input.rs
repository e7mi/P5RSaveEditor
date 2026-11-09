//! Number and text normalisation shared by the editor fields.

/// Converts full-width ASCII, full-width spaces and common separators to plain input.
pub fn normalize_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        let code = ch as u32;
        let mapped = match code {
            0x3000 => ' ',
            0xFF01..=0xFF5E => char::from_u32(code - 0xFEE0).unwrap_or(ch),
            0xFFE5 => ' ',
            _ => ch,
        };
        out.push(mapped);
    }
    out
}

/// Parses an unsigned integer, allowing commas, spaces and full-width digits.
pub fn parse_u32(text: &str) -> Option<u32> {
    let normalized = normalize_text(text);
    let cleaned: String = normalized
        .chars()
        .filter(|ch| !ch.is_whitespace() && *ch != ',' && *ch != '_' && *ch != '¥' && *ch != '￥')
        .collect();
    if cleaned.is_empty() {
        return None;
    }
    cleaned.parse().ok()
}

pub fn parse_u16(text: &str) -> Option<u16> {
    parse_u32(text).and_then(|value| u16::try_from(value).ok())
}

pub fn parse_u8(text: &str) -> Option<u8> {
    parse_u32(text).and_then(|value| u8::try_from(value).ok())
}

/// Formats minutes as the `HH:MM` shape used by the single playtime field.
pub fn format_playtime_input(minutes: u32) -> String {
    format!("{}:{:02}", minutes / 60, minutes % 60)
}

/// Parses `87:09`, `87時間9分`, `87h9m` or a bare minute count.
pub fn parse_playtime_minutes(text: &str) -> Option<u32> {
    let normalized = normalize_text(text);
    let trimmed = normalized.trim();
    if trimmed.is_empty() {
        return None;
    }
    let cleaned = trimmed
        .replace("時間", ":")
        .replace(['h', 'H'], ":")
        .replace(['分', 'm', 'M'], "");
    if !cleaned.contains(':') {
        return parse_u32(&cleaned);
    }
    let mut parts = cleaned.split(':');
    let hours: u32 = parts.next()?.trim().parse().ok()?;
    let minutes: u32 = parts.next()?.trim().parse().ok()?;
    if parts.next().is_some() || minutes >= 60 {
        return None;
    }
    hours.checked_mul(60)?.checked_add(minutes)
}

pub fn playtime_text(minutes: u32) -> String {
    format!("{}時間{:02}分", minutes / 60, minutes % 60)
}

pub fn money_text(value: u32) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 + 1);
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    format!("¥{out}")
}

pub fn difficulty_label(value: u8) -> &'static str {
    match value {
        0 => "安全",
        1 => "易しい",
        2 => "普通",
        3 => "難しい",
        4 => "容赦なし",
        _ => "不明",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_numbers_with_formatting() {
        assert_eq!(parse_u32("1,234,567"), Some(1_234_567));
        assert_eq!(parse_u32("１２３４"), Some(1234));
        assert_eq!(parse_u32("￥1,234"), Some(1234));
        assert_eq!(parse_u32("  99 "), Some(99));
        assert_eq!(parse_u32(""), None);
        assert_eq!(parse_u32("12a"), None);
    }

    #[test]
    fn parses_playtime_variants() {
        assert_eq!(parse_playtime_minutes("87:09"), Some(87 * 60 + 9));
        assert_eq!(parse_playtime_minutes("87時間9分"), Some(87 * 60 + 9));
        assert_eq!(parse_playtime_minutes("87h9m"), Some(87 * 60 + 9));
        assert_eq!(parse_playtime_minutes("5229"), Some(5229));
        assert_eq!(parse_playtime_minutes("87:60"), None);
        assert_eq!(parse_playtime_minutes(""), None);
    }

    #[test]
    fn formats_playtime() {
        assert_eq!(format_playtime_input(5229), "87:09");
        assert_eq!(playtime_text(5229), "87時間09分");
    }

    #[test]
    fn formats_money() {
        assert_eq!(money_text(9_999_999), "¥9,999,999");
        assert_eq!(money_text(0), "¥0");
    }
}
