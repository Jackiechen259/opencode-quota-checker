//! Small serde/date helpers shared by the Console usage parsers.
//!
//! The Console serializes `BigInt` values as decimal strings. Parsers accept
//! those strings or plain JSON integers and reject negative, fractional, and
//! overflowing values instead of silently rounding them.

use serde::de::{self, Visitor};
use serde::Deserializer;
use std::fmt;

/// Accepts the Console's decimal strings and plain JSON integers.
pub(crate) fn deserialize_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    struct U64Visitor;

    impl<'de> Visitor<'de> for U64Visitor {
        type Value = u64;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a non-negative integer or a decimal integer string")
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
            Ok(value)
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            u64::try_from(value).map_err(|_| E::custom("expected a non-negative integer"))
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            value
                .parse::<u64>()
                .map_err(|_| E::custom("expected a decimal integer string"))
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            self.visit_str(&value)
        }
    }

    deserializer.deserialize_any(U64Visitor)
}

/// Parses an RFC 3339 instant or a `YYYY-MM-DD` UTC date into epoch millis.
///
/// The Go endpoints return `Date` values serialized as RFC 3339 strings, while
/// some activity windows use date-only values. Both are accepted; anything
/// else is a contract error.
pub(crate) fn parse_instant(value: &str) -> Result<i64, String> {
    if let Ok(datetime) = chrono::DateTime::parse_from_rfc3339(value) {
        return Ok(datetime.timestamp_millis());
    }
    if let Ok(date) = chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        let datetime = date
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| format!("invalid UTC date: {value}"))?;
        return Ok(datetime.and_utc().timestamp_millis());
    }
    Err(format!("invalid instant: {value}"))
}

/// Normalizes a `YYYY-MM-DD` date (a full ISO timestamp is accepted and
/// truncated to its date part) while rejecting invalid calendar dates.
pub(crate) fn normalize_date(value: &str) -> Result<String, String> {
    let date = value
        .get(0..10)
        .ok_or_else(|| format!("invalid date: {value}"))?;
    chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| format!("invalid date: {value}"))?;
    Ok(date.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rfc3339_and_date_only_instants() {
        assert_eq!(
            parse_instant("2026-10-26T00:00:00.000Z").expect("rfc3339"),
            1_792_972_800_000
        );
        assert_eq!(parse_instant("1970-01-01").expect("date only"), 0);
        assert!(parse_instant("not-a-date").is_err());
    }

    #[test]
    fn normalizes_dates_and_rejects_impossible_calendar_values() {
        assert_eq!(
            normalize_date("2026-10-26").expect("valid date"),
            "2026-10-26"
        );
        assert_eq!(
            normalize_date("2026-10-26T12:00:00Z").expect("timestamp date"),
            "2026-10-26"
        );
        assert!(normalize_date("2026-02-30").is_err());
        assert!(normalize_date("26/10/2026").is_err());
    }
}
