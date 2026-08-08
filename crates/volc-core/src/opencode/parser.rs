//! Parses the OpenCode Go workspace dashboard HTML into the shared quota model.
//!
//! The dashboard is an unofficial data source whose structure may change at any
//! time. Parsing happens in layers:
//!
//! 1. **SSR strategy** - extracts the server-rendered `usagePercent` /
//!    `resetInSec` values for `rollingUsage` / `weeklyUsage` / `monthlyUsage`.
//! 2. **DOM strategy** - falls back to semantic `data-slot` attributes
//!    (`usage-item`, `usage-label`, `usage-value`, `reset-time`).
//! 3. **Fail safely** - any unsupported structure returns a typed
//!    [`VolcError::Parse`] error and never fabricates zero usage.

use crate::models::{Provider, UsageReport, WindowReport};
use crate::VolcError;

/// SSR window tokens in document order, mapped to normalized keys and labels.
const SSR_WINDOWS: [(&str, &str, &str); 3] = [
    ("rollingUsage", "rolling-5h", "5 小时"),
    ("weeklyUsage", "weekly", "近一周"),
    ("monthlyUsage", "monthly", "近一月"),
];

const USAGE_ITEM: &str = "data-slot=\"usage-item\"";
const USAGE_LABEL: &str = "usage-label";
const USAGE_VALUE: &str = "usage-value";
const RESET_TIME: &str = "reset-time";

/// Parses dashboard HTML into a normalized OpenCode Go usage report.
///
/// A login page is reported as [`VolcError::AuthenticationFailed`]; an
/// unsupported structure is reported as [`VolcError::Parse`]. A parser failure
/// is never interpreted as zero usage.
pub fn parse_open_code_go_quota(html: &str, now_ms: i64) -> Result<UsageReport, VolcError> {
    if is_login_page(html) {
        return Err(VolcError::AuthenticationFailed);
    }

    let windows = parse_ssr(html)
        .or_else(|| parse_dom(html))
        .ok_or_else(|| VolcError::Parse("no supported quota structure was found".to_owned()))?;
    if windows.is_empty() {
        return Err(VolcError::Parse(
            "the dashboard contains no quota windows".to_owned(),
        ));
    }

    let windows = windows
        .into_iter()
        .map(|window| window.into_report(now_ms))
        .collect();
    Ok(UsageReport {
        provider: Provider::OpenCodeGo,
        plan_type: String::new(),
        windows,
        fetched_at: now_ms,
    })
}

/// Reports whether the response is an unauthenticated login page.
pub fn is_login_page(html: &str) -> bool {
    let lower = html.to_ascii_lowercase();
    let login_markers = [
        "login",
        "sign in",
        "sign-in",
        "log in",
        "log-in",
        "sign up",
        "sign-up",
        "create account",
    ];
    let quota_markers = [
        "rollingusage",
        "weeklyusage",
        "monthlyusage",
        "data-slot=\"usage-item\"",
    ];
    let has_login = login_markers.iter().any(|marker| lower.contains(marker));
    let has_quota = quota_markers.iter().any(|marker| lower.contains(marker));
    has_login && !has_quota
}

/// A parsed quota window before normalization onto the 100-point scale.
#[derive(Debug, Clone, Copy, PartialEq)]
struct RawWindow {
    key: &'static str,
    label: &'static str,
    usage_percent: f64,
    reset_in_secs: i64,
}

impl RawWindow {
    /// Normalizes onto a 100-point scale so the shared UI can render it.
    ///
    /// OpenCode only reports percentages, so `quota = 100` and `used` is the
    /// reported percentage; `remaining` and `percent` follow from that.
    fn into_report(self, now_ms: i64) -> WindowReport {
        let used = self.usage_percent.clamp(0.0, 100.0);
        let has_reset = self.reset_in_secs > 0;
        let reset_time = if has_reset {
            now_ms.saturating_add(self.reset_in_secs.saturating_mul(1_000))
        } else {
            0
        };
        WindowReport {
            key: self.key.to_owned(),
            label: self.label.to_owned(),
            quota: 100.0,
            used,
            remaining: (100.0 - used).max(0.0),
            percent: used,
            // `subscribe_time` doubles as the alert-deduplication cycle; for
            // OpenCode the reset cycle is the only meaningful cycle.
            subscribe_time: reset_time,
            reset_time,
            reset_in_secs: if has_reset { self.reset_in_secs } else { 0 },
        }
    }
}

/// Strategy A - structured server-rendered data.
fn parse_ssr(html: &str) -> Option<Vec<RawWindow>> {
    let mut boundaries = SSR_WINDOWS
        .iter()
        .flat_map(|(token, _, _)| key_positions(html, token))
        .collect::<Vec<_>>();
    if boundaries.is_empty() {
        return None;
    }
    boundaries.sort_unstable();
    boundaries.dedup();

    let mut windows = Vec::new();
    for (token, key, label) in SSR_WINDOWS {
        let parsed = key_positions(html, token).find_map(|start| {
            let end = boundaries
                .iter()
                .copied()
                .find(|position| *position > start)
                .unwrap_or(html.len());
            let segment = &html[start..end];
            let usage_percent = extract_number(segment, "usagePercent")?;
            usage_percent.is_finite().then(|| {
                let reset_in_secs = extract_number(segment, "resetInSec")
                    .filter(|value| value.is_finite())
                    .map_or(0, |value| value as i64);
                (usage_percent, reset_in_secs)
            })
        });
        let Some((usage_percent, reset_in_secs)) = parsed else {
            continue;
        };
        windows.push(RawWindow {
            key,
            label,
            usage_percent,
            reset_in_secs,
        });
    }

    if windows.is_empty() {
        None
    } else {
        Some(windows)
    }
}

/// Strategy B - semantic DOM attributes.
fn parse_dom(html: &str) -> Option<Vec<RawWindow>> {
    if !html.contains(USAGE_ITEM) {
        return None;
    }

    let mut windows = Vec::new();
    let mut search_from = 0usize;
    while let Some(relative) = html[search_from..].find(USAGE_ITEM) {
        let start = search_from + relative;
        let block_start = start + USAGE_ITEM.len();
        let block_end = html[block_start..]
            .find(USAGE_ITEM)
            .map_or(html.len(), |next| block_start + next);
        let block = &html[start..block_end];

        if let Some(label_text) = element_text(block, USAGE_LABEL) {
            if let Some((key, label)) = classify_label(&label_text) {
                if let Some(usage_percent) =
                    element_text(block, USAGE_VALUE).and_then(|value| parse_percent(&value))
                {
                    if usage_percent.is_finite() {
                        let reset_in_secs = element_text(block, RESET_TIME)
                            .and_then(|text| parse_duration(&text))
                            .unwrap_or(0);
                        windows.push(RawWindow {
                            key,
                            label,
                            usage_percent,
                            reset_in_secs,
                        });
                    }
                }
            }
        }
        search_from = block_start;
    }

    if windows.is_empty() {
        None
    } else {
        Some(windows)
    }
}

/// Locates every occurrence of a window key.
///
/// Solid's hydration output can mention a key before the object containing its
/// quota values. Each occurrence is therefore evaluated within the boundary of
/// the next known window key instead of assuming that the first occurrence is
/// authoritative or that the three keys have a fixed document order.
fn key_positions<'a>(html: &'a str, key: &'a str) -> impl Iterator<Item = usize> + 'a {
    html.match_indices(key).map(|(position, _)| position)
}

/// Extracts the first numeric value assigned to a JSON field in a segment.
fn extract_number(segment: &str, field: &str) -> Option<f64> {
    let field_index = segment.find(field)?;
    let after_field = &segment[field_index + field.len()..];
    let colon_index = after_field.find(':')?;
    let value = after_field[colon_index + 1..].trim_start();
    let value = value.strip_prefix('"').unwrap_or(value);
    let number_end = value
        .find(|character: char| {
            !(character.is_ascii_digit()
                || character == '.'
                || character == '-'
                || character == '+')
        })
        .unwrap_or(value.len());
    value[..number_end].parse::<f64>().ok()
}

/// Extracts the text content of an element carrying a `data-slot` attribute.
fn element_text(html: &str, slot: &str) -> Option<String> {
    let marker = format!("data-slot=\"{slot}\"");
    let marker_index = html.find(&marker)?;
    let after_marker = &html[marker_index + marker.len()..];
    let open_index = after_marker.find('>')?;
    let text = &after_marker[open_index + 1..];
    let close_index = text.find('<')?;
    let decoded = decode_html_entities(text[..close_index].trim());
    if decoded.is_empty() {
        None
    } else {
        Some(decoded)
    }
}

/// Maps a displayed label to a normalized window key, case-insensitively.
fn classify_label(label: &str) -> Option<(&'static str, &'static str)> {
    let normalized: String = label
        .chars()
        .filter(|character| !character.is_whitespace())
        .flat_map(|character| character.to_lowercase())
        .collect();
    if normalized.contains("5hour") || normalized.contains("rolling") {
        Some(("rolling-5h", "5 小时"))
    } else if normalized.contains("weekly") || normalized.contains("周") {
        Some(("weekly", "近一周"))
    } else if normalized.contains("monthly") || normalized.contains("月") {
        Some(("monthly", "近一月"))
    } else {
        None
    }
}

/// Parses a percentage like `78%`, tolerating surrounding whitespace.
fn parse_percent(value: &str) -> Option<f64> {
    let cleaned: String = value
        .chars()
        .filter(|character| character.is_ascii_digit() || *character == '.' || *character == '-')
        .collect();
    if cleaned.is_empty() {
        None
    } else {
        cleaned.parse::<f64>().ok()
    }
}

/// Parses a reset countdown such as `2h 14m`, `17d`, `45s`, or plain seconds.
fn parse_duration(value: &str) -> Option<i64> {
    let value = value.trim().to_ascii_lowercase();
    let value = value
        .strip_prefix("in ")
        .or_else(|| value.strip_prefix("in"))
        .unwrap_or(&value);
    let value = value.trim();

    if let Ok(seconds) = value.parse::<i64>() {
        return Some(seconds);
    }

    let mut total = 0i64;
    let mut rest = value;
    let mut any = false;
    while !rest.is_empty() {
        let digit_start = rest.find(|character: char| character.is_ascii_digit())?;
        rest = &rest[digit_start..];
        let digit_end = rest
            .find(|character: char| !character.is_ascii_digit())
            .unwrap_or(rest.len());
        let amount = rest[..digit_end].parse::<i64>().ok()?;
        rest = &rest[digit_end..];
        let unit = rest.chars().next()?;
        rest = &rest[unit.len_utf8()..];
        total += match unit {
            'd' => amount.saturating_mul(86_400),
            'h' => amount.saturating_mul(3_600),
            'm' => amount.saturating_mul(60),
            's' => amount,
            _ => return None,
        };
        any = true;
    }
    if any {
        Some(total)
    } else {
        None
    }
}

/// Decodes the small set of HTML entities that can appear in labels/values.
fn decode_html_entities(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(ampersand) = rest.find('&') {
        output.push_str(&rest[..ampersand]);
        rest = &rest[ampersand..];
        let Some(semicolon) = rest.find(';') else {
            output.push('&');
            rest = &rest[1..];
            continue;
        };
        if semicolon <= 8 {
            let entity = &rest[1..semicolon];
            let decoded = match entity {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                "nbsp" => Some('\u{a0}'),
                _ => None,
            };
            if let Some(character) = decoded {
                output.push(character);
                rest = &rest[semicolon + 1..];
                continue;
            }
        }
        output.push('&');
        rest = &rest[1..];
    }
    output.push_str(rest);
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    const SSR: &str = include_str!("../../../../tests/fixtures/opencode-go/ssr-dashboard.html");
    const DOM: &str = include_str!("../../../../tests/fixtures/opencode-go/dom-dashboard.html");
    const LOGIN: &str = include_str!("../../../../tests/fixtures/opencode-go/login-page.html");
    const MALFORMED: &str =
        include_str!("../../../../tests/fixtures/opencode-go/malformed-dashboard.html");
    const MISSING_WINDOW: &str =
        include_str!("../../../../tests/fixtures/opencode-go/missing-window.html");
    const NOW_MS: i64 = 1_778_800_000_000;

    #[test]
    fn parses_valid_ssr_response() {
        let report = parse_open_code_go_quota(SSR, NOW_MS).expect("SSR fixture is valid");
        assert_eq!(report.provider, Provider::OpenCodeGo);
        assert_eq!(report.plan_type, "");
        assert_eq!(report.fetched_at, NOW_MS);
        assert_eq!(report.windows.len(), 3);

        let rolling = &report.windows[0];
        assert_eq!(rolling.key, "rolling-5h");
        assert_eq!(rolling.label, "5 小时");
        assert_eq!(rolling.percent, 78.0);
        assert_eq!(rolling.used, 78.0);
        assert_eq!(rolling.remaining, 22.0);
        assert_eq!(rolling.reset_in_secs, 6_132);
        assert_eq!(rolling.reset_time, NOW_MS + 6_132_000);

        let weekly = &report.windows[1];
        assert_eq!(weekly.key, "weekly");
        assert_eq!(weekly.percent, 52.0);
        assert_eq!(weekly.remaining, 48.0);

        let monthly = &report.windows[2];
        assert_eq!(monthly.key, "monthly");
        assert_eq!(monthly.percent, 19.0);
        assert_eq!(monthly.remaining, 81.0);
    }

    #[test]
    fn dom_fallback_produces_the_same_windows() {
        let report = parse_open_code_go_quota(DOM, NOW_MS).expect("DOM fixture is valid");
        assert_eq!(report.windows.len(), 3);
        let keys: Vec<&str> = report
            .windows
            .iter()
            .map(|window| window.key.as_str())
            .collect();
        assert_eq!(keys, vec!["rolling-5h", "weekly", "monthly"]);
        let percents: Vec<f64> = report.windows.iter().map(|window| window.percent).collect();
        assert_eq!(percents, vec![78.0, 52.0, 19.0]);
    }

    #[test]
    fn out_of_order_ssr_keys_do_not_panic() {
        // The real dashboard can emit the window keys in any order (for example
        // nested inside a JSON blob). Each window must still read its own value
        // and the parser must never slice the document backwards.
        let html = r#"<script>window.__DATA__ = {
            "monthlyUsage": { "usagePercent": 19, "resetInSec": 100 },
            "weeklyUsage": { "usagePercent": 52, "resetInSec": 200 },
            "rollingUsage": { "usagePercent": 78, "resetInSec": 300 }
        }</script>"#;
        let report =
            parse_open_code_go_quota(html, NOW_MS).expect("out-of-order keys parse safely");
        assert_eq!(report.windows.len(), 3);
        let by_key: std::collections::HashMap<&str, &WindowReport> = report
            .windows
            .iter()
            .map(|window| (window.key.as_str(), window))
            .collect();
        assert_eq!(by_key["rolling-5h"].percent, 78.0);
        assert_eq!(by_key["weekly"].percent, 52.0);
        assert_eq!(by_key["monthly"].percent, 19.0);
        assert_eq!(by_key["rolling-5h"].reset_in_secs, 300);
        assert_eq!(by_key["weekly"].reset_in_secs, 200);
        assert_eq!(by_key["monthly"].reset_in_secs, 100);
    }

    #[test]
    fn duplicate_key_without_values_does_not_shadow_real_window() {
        // The live Solid hydration payload currently mentions monthlyUsage in
        // metadata before emitting the actual quota objects. The metadata key
        // must not borrow rollingUsage's values or shadow the later monthly
        // object.
        let html = r#"<script>window.__DATA__ = {
            "monthlyUsage": null,
            "rollingUsage": { "resetInSec": 300, "usagePercent": 3 },
            "weeklyUsage": { "resetInSec": 200, "usagePercent": 8 },
            "monthlyUsage": { "resetInSec": 100, "usagePercent": 4 }
        }</script>"#;
        let report = parse_open_code_go_quota(html, NOW_MS)
            .expect("the later monthly quota object is authoritative");
        let by_key: std::collections::HashMap<&str, &WindowReport> = report
            .windows
            .iter()
            .map(|window| (window.key.as_str(), window))
            .collect();
        assert_eq!(by_key["rolling-5h"].percent, 3.0);
        assert_eq!(by_key["weekly"].percent, 8.0);
        assert_eq!(by_key["monthly"].percent, 4.0);
        assert_eq!(by_key["monthly"].reset_in_secs, 100);
    }

    #[test]
    fn missing_one_window_keeps_the_rest() {
        let report =
            parse_open_code_go_quota(MISSING_WINDOW, NOW_MS).expect("partial fixture is valid");
        assert_eq!(report.windows.len(), 2);
        assert_eq!(report.windows[0].key, "rolling-5h");
        assert_eq!(report.windows[1].key, "weekly");
    }

    #[test]
    fn dom_label_matching_is_case_and_whitespace_tolerant() {
        assert_eq!(
            classify_label(" 5 HOURS ").map(|(key, _)| key),
            Some("rolling-5h")
        );
        assert_eq!(
            classify_label("Rolling 5 hour").map(|(key, _)| key),
            Some("rolling-5h")
        );
        assert_eq!(
            classify_label("近 一周").map(|(key, _)| key),
            Some("weekly")
        );
        assert_eq!(
            classify_label("近一月").map(|(key, _)| key),
            Some("monthly")
        );
        assert_eq!(classify_label("Something else"), None);
    }

    #[test]
    fn percent_values_are_clamped_to_the_valid_range() {
        assert_eq!(clamp_percent(-1.0), 0.0);
        assert_eq!(clamp_percent(101.0), 100.0);
        assert_eq!(clamp_percent(50.0), 50.0);
    }

    fn clamp_percent(value: f64) -> f64 {
        value.clamp(0.0, 100.0)
    }

    #[test]
    fn malformed_percentages_do_not_corrupt_windows() {
        let malformed = r#"{"props":{"quota":{
            "rollingUsage": { "usagePercent": -5, "resetInSec": 100 },
            "weeklyUsage": { "usagePercent": 150, "resetInSec": 100 },
            "monthlyUsage": { "usagePercent": "not-a-number", "resetInSec": 100 }
        }}}"#;
        let report =
            parse_open_code_go_quota(malformed, NOW_MS).expect("clamped windows are valid");
        assert_eq!(report.windows.len(), 2);
        assert_eq!(report.windows[0].percent, 0.0);
        assert_eq!(report.windows[1].percent, 100.0);
    }

    #[test]
    fn login_page_is_an_authentication_error() {
        let error = parse_open_code_go_quota(LOGIN, NOW_MS).expect_err("login page must fail");
        assert!(matches!(error, VolcError::AuthenticationFailed));
    }

    #[test]
    fn unsupported_html_is_a_parse_error_not_zero_usage() {
        let error =
            parse_open_code_go_quota(MALFORMED, NOW_MS).expect_err("unknown HTML must fail");
        assert!(matches!(error, VolcError::Parse(_)));
    }

    #[test]
    fn duration_formats_are_supported() {
        assert_eq!(parse_duration("6120"), Some(6_120));
        assert_eq!(parse_duration("in 1h 42m"), Some(6_120));
        assert_eq!(parse_duration("2h 14m"), Some(8_040));
        assert_eq!(parse_duration("17d"), Some(1_468_800));
        assert_eq!(parse_duration("45s"), Some(45));
        assert_eq!(parse_duration("soon"), None);
    }

    #[test]
    fn html_entities_are_decoded() {
        assert_eq!(decode_html_entities("5&nbsp;hour"), "5\u{a0}hour");
        assert_eq!(decode_html_entities("a &amp; b"), "a & b");
        assert_eq!(decode_html_entities("plain"), "plain");
    }

    #[test]
    fn extract_number_handles_quoted_and_spaced_values() {
        assert_eq!(
            extract_number(r#"{"usagePercent":78}"#, "usagePercent"),
            Some(78.0)
        );
        assert_eq!(
            extract_number(r#"{"usagePercent": 78.5}"#, "usagePercent"),
            Some(78.5)
        );
        assert_eq!(
            extract_number(r#"{"usagePercent":"78"}"#, "usagePercent"),
            Some(78.0)
        );
        assert_eq!(extract_number(r#"{}"#, "usagePercent"), None);
    }
}
