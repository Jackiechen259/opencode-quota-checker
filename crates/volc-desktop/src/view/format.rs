use chrono::{DateTime, Utc};

/// Formats a Unix millisecond timestamp for display.
pub fn timestamp(timestamp_ms: i64) -> String {
    DateTime::<Utc>::from_timestamp_millis(timestamp_ms).map_or_else(
        || "未知时间".to_owned(),
        |value| value.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
    )
}

/// Formats a signed number of seconds as a reset countdown.
pub fn countdown(seconds: i64) -> String {
    if seconds <= 0 {
        return "即将重置".to_owned();
    }
    let days = seconds / 86_400;
    let hours = (seconds % 86_400) / 3_600;
    let minutes = (seconds % 3_600) / 60;
    if days > 0 {
        format!("{days} 天 {hours} 小时")
    } else if hours > 0 {
        format!("{hours} 小时 {minutes} 分")
    } else {
        format!("{minutes} 分 {} 秒", seconds % 60)
    }
}

/// Short countdown for compact displays (e.g. `04:54` or `1天`).
pub fn countdown_short(seconds: i64) -> String {
    if seconds <= 0 {
        return "即将重置".to_owned();
    }
    let days = seconds / 86_400;
    let hours = (seconds % 86_400) / 3_600;
    let minutes = (seconds % 3_600) / 60;
    let secs = seconds % 60;
    if days > 0 {
        format!("{days}天 {hours}时")
    } else if hours > 0 {
        format!("{hours:02}:{minutes:02}")
    } else {
        format!("{minutes:02}:{secs:02}")
    }
}

/// Human-friendly "time ago" relative to `now_ms`.
pub fn relative(fetched_at_ms: i64, now_ms: i64) -> String {
    let delta = now_ms.saturating_sub(fetched_at_ms).max(0) / 1_000;
    if delta < 5 {
        return "刚刚".to_owned();
    }
    if delta < 60 {
        return format!("{delta} 秒前");
    }
    let minutes = delta / 60;
    if minutes < 60 {
        return format!("{minutes} 分钟前");
    }
    let hours = minutes / 60;
    if hours < 24 {
        return format!("{hours} 小时前");
    }
    let days = hours / 24;
    format!("{days} 天前")
}

/// Formats a number with thousands separators and one decimal place.
pub fn number(value: f64) -> String {
    let rounded = (value * 10.0).round() / 10.0;
    let mut s = format!("{rounded:.1}");
    let (int_part, frac) = s.split_once('.').unwrap_or((&s, ""));
    let int = int_part
        .chars()
        .rev()
        .collect::<Vec<_>>()
        .chunks(3)
        .map(|c| c.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(",");
    let int = int.chars().rev().collect::<String>();
    s = if frac.is_empty() {
        int
    } else {
        format!("{int}.{frac}")
    };
    s
}

/// Formats a percentage with one decimal, tolerating values over 100.
pub fn percent(value: f64) -> String {
    format!("{:.1}%", value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn countdown_handles_boundaries() {
        assert_eq!(countdown(-1), "即将重置");
        assert_eq!(countdown(59), "0 分 59 秒");
        assert_eq!(countdown(3_661), "1 小时 1 分");
        assert_eq!(countdown(90_000), "1 天 1 小时");
    }

    #[test]
    fn invalid_timestamp_is_safe() {
        assert_eq!(timestamp(i64::MAX), "未知时间");
    }

    #[test]
    fn number_uses_thousands_separators() {
        assert_eq!(number(75780.9), "75,780.9");
        assert_eq!(number(100000.0), "100,000.0");
        assert_eq!(number(24219.1), "24,219.1");
        assert_eq!(number(0.0), "0.0");
    }

    #[test]
    fn relative_descends_gracefully() {
        assert_eq!(relative(0, 0), "刚刚");
        assert_eq!(relative(0, 3_000), "刚刚");
        assert_eq!(relative(0, 30_000), "30 秒前");
        assert_eq!(relative(0, 120_000), "2 分钟前");
    }
}
