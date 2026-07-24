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
}
