//! Text formatting shared by the views (ports of `src/lib/formatters.ts` and `tauri.ts` helpers).
use chrono::{DateTime, Local, TimeZone};

/// `yyyy-MM-dd HH:mm:ss` in local time; `None` → "-", unparseable → unchanged.
pub fn format_timestamp(ts: Option<&str>) -> String {
    format_timestamp_in(ts, &Local)
}

pub fn format_timestamp_in<Tz: TimeZone>(ts: Option<&str>, tz: &Tz) -> String
where
    Tz::Offset: std::fmt::Display,
{
    fmt_ts(ts, tz, "%Y-%m-%d %H:%M:%S")
}

/// `HH:mm:ss` in local time, same fallbacks as `format_timestamp`.
pub fn format_short_timestamp(ts: Option<&str>) -> String {
    format_short_timestamp_in(ts, &Local)
}

pub fn format_short_timestamp_in<Tz: TimeZone>(ts: Option<&str>, tz: &Tz) -> String
where
    Tz::Offset: std::fmt::Display,
{
    fmt_ts(ts, tz, "%H:%M:%S")
}

fn fmt_ts<Tz: TimeZone>(ts: Option<&str>, tz: &Tz, fmt: &str) -> String
where
    Tz::Offset: std::fmt::Display,
{
    let Some(ts) = ts.filter(|s| !s.is_empty()) else {
        return "-".into();
    };
    match DateTime::parse_from_rfc3339(ts) {
        Ok(d) => d.with_timezone(tz).format(fmt).to_string(),
        Err(_) => ts.to_string(),
    }
}

/// `toLocaleString()` grouping for en-US: 1234567 → "1,234,567".
pub fn thousands(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// First case-insensitive match as (before, match, after); `None` for an empty keyword or no match.
pub fn highlight_match(text: &str, keyword: &str) -> Option<(String, String, String)> {
    if keyword.is_empty() {
        return None;
    }
    let needle = keyword.to_lowercase();
    let n = keyword.chars().count();
    // ponytail: O(len * keyword) scan; fine for single log lines.
    for (start, _) in text.char_indices() {
        let end = text[start..].char_indices().nth(n).map_or(text.len(), |(i, _)| start + i);
        if text[start..end].to_lowercase() == needle {
            return Some((text[..start].into(), text[start..end].into(), text[end..].into()));
        }
    }
    None
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum TimeRange {
    M15,
    M30,
    H1,
    H3,
    H6,
    H12,
    H24,
    H48,
}

impl TimeRange {
    pub const ALL: [TimeRange; 8] = [
        Self::M15,
        Self::M30,
        Self::H1,
        Self::H3,
        Self::H6,
        Self::H12,
        Self::H24,
        Self::H48,
    ];

    pub fn seconds(self) -> i64 {
        match self {
            Self::M15 => 900,
            Self::M30 => 1800,
            Self::H1 => 3600,
            Self::H3 => 10800,
            Self::H6 => 21600,
            Self::H12 => 43200,
            Self::H24 => 86400,
            Self::H48 => 172800,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::M15 => "Last 15 minutes",
            Self::M30 => "Last 30 minutes",
            Self::H1 => "Last 1 hour",
            Self::H3 => "Last 3 hours",
            Self::H6 => "Last 6 hours",
            Self::H12 => "Last 12 hours",
            Self::H24 => "Last 24 hours",
            Self::H48 => "Last 48 hours",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::M15 => "15m",
            Self::M30 => "30m",
            Self::H1 => "1h",
            Self::H3 => "3h",
            Self::H6 => "6h",
            Self::H12 => "12h",
            Self::H24 => "24h",
            Self::H48 => "48h",
        }
    }
}

pub const REFRESH_INTERVALS_MS: [u32; 5] = [5000, 10000, 15000, 20000, 30000];

pub fn refresh_label(ms: u32) -> String {
    format!("{} seconds", ms / 1000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_values() {
        let utc = chrono::Utc;
        assert_eq!(format_timestamp_in(Some("2026-01-09T17:22:13Z"), &utc), "2026-01-09 17:22:13");
        assert_eq!(
            format_timestamp_in(Some("2026-01-09T17:22:13Z"), &chrono::FixedOffset::east_opt(7 * 3600).unwrap()),
            "2026-01-10 00:22:13"
        );
        assert_eq!(format_timestamp_in(None, &utc), "-");
        assert_eq!(format_timestamp_in(Some("not a date"), &utc), "not a date");
        assert_eq!(format_short_timestamp_in(Some("2026-01-09T17:22:13.123456Z"), &utc), "17:22:13");
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(1234567), "1,234,567");
        assert_eq!(thousands(999), "999");
        assert_eq!(highlight_match("Hello World", "WORLD"), Some(("Hello ".into(), "World".into(), "".into())));
        assert_eq!(highlight_match("abc", ""), None);
        assert_eq!(highlight_match("abc", "x"), None);
        assert_eq!(TimeRange::H48.seconds(), 172800);
        assert_eq!(TimeRange::M30.label(), "Last 30 minutes");
    }
}
