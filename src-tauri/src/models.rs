use serde::{Deserialize, Serialize};

/// GetAFPUsage 完整响应
#[derive(Debug, Clone, Deserialize)]
pub struct AfpResponse {
    #[serde(default, rename = "ResponseMetadata")]
    pub response_metadata: Option<ResponseMetadata>,
    #[serde(default, rename = "Result")]
    pub result: Option<AfpResult>,
    #[serde(default, rename = "Error")]
    pub error: Option<VolcError>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResponseMetadata {
    #[serde(default, rename = "RequestId")]
    pub request_id: Option<String>,
    #[serde(default, rename = "Action")]
    pub action: Option<String>,
    #[serde(default, rename = "Version")]
    pub version: Option<String>,
    #[serde(default, rename = "Service")]
    pub service: Option<String>,
    #[serde(default, rename = "Region")]
    pub region: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VolcError {
    #[serde(default, rename = "Code")]
    pub code: Option<String>,
    #[serde(default, rename = "Message")]
    pub message: Option<String>,
    #[serde(default, rename = "CodeN")]
    pub code_n: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AfpResult {
    #[serde(default, rename = "PlanType")]
    pub plan_type: String,
    #[serde(default, rename = "AFPFiveHour")]
    pub five_hour: Option<Window>,
    #[serde(default, rename = "AFPDaily")]
    pub daily: Option<Window>,
    #[serde(default, rename = "AFPWeekly")]
    pub weekly: Option<Window>,
    #[serde(default, rename = "AFPMonthly")]
    pub monthly: Option<Window>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Window {
    #[serde(rename = "Quota")]
    pub quota: f64,
    #[serde(rename = "Used")]
    pub used: f64,
    #[serde(rename = "SubscribeTime")]
    pub subscribe_time: i64,
    #[serde(rename = "ResetTime")]
    pub reset_time: i64,
}

impl Window {
    pub fn remaining(&self) -> f64 {
        (self.quota - self.used).max(0.0)
    }
    pub fn percent(&self) -> f64 {
        if self.quota > 0.0 {
            (self.used / self.quota * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        }
    }
    /// 距离重置还有多少秒(<0 表示已过期)
    pub fn reset_in_secs(&self, now_ms: i64) -> i64 {
        (self.reset_time - now_ms) / 1000
    }
}

/// 透传给前端的窗口信息
#[derive(Debug, Clone, Serialize)]
pub struct WindowReport {
    pub key: String,
    pub label: String,
    pub quota: f64,
    pub used: f64,
    pub remaining: f64,
    pub percent: f64,
    pub subscribe_time: i64,
    pub reset_time: i64,
    pub reset_in_secs: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageReport {
    pub plan_type: String,
    pub windows: Vec<WindowReport>,
    pub fetched_at: i64,
}

impl AfpResult {
    pub fn to_report(&self) -> UsageReport {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let entries: [(&str, &str, Option<&Window>); 3] = [
            ("five_hour", "5 小时", self.five_hour.as_ref()),
            ("weekly", "近一周", self.weekly.as_ref()),
            ("monthly", "近一月", self.monthly.as_ref()),
        ];
        let windows = entries
            .iter()
            .filter_map(|(key, label, w)| {
                w.map(|win| WindowReport {
                    key: key.to_string(),
                    label: label.to_string(),
                    quota: win.quota,
                    used: win.used,
                    remaining: win.remaining(),
                    percent: win.percent(),
                    subscribe_time: win.subscribe_time,
                    reset_time: win.reset_time,
                    reset_in_secs: win.reset_in_secs(now_ms),
                })
            })
            .collect();
        UsageReport {
            plan_type: self.plan_type.clone(),
            windows,
            fetched_at: now_ms,
        }
    }
}

impl AfpResponse {
    pub fn into_report(self) -> Result<UsageReport, String> {
        if let Some(err) = self.error {
            return Err(format!(
                "[{}] {}",
                err.code.unwrap_or_else(|| "Unknown".into()),
                err.message.unwrap_or_else(|| "无错误信息".into())
            ));
        }
        match self.result {
            Some(r) => Ok(r.to_report()),
            None => Err("响应缺少 Result 字段".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
  "ResponseMetadata": {
    "RequestId": "20260511034034A1B2C3D4E5F60718293",
    "Action": "GetAFPUsage",
    "Version": "2024-01-01",
    "Service": "ark",
    "Region": "cn-beijing"
  },
  "Result": {
    "PlanType": "Large",
    "AFPFiveHour": {
      "Quota": 50.0,
      "Used": 12.5,
      "SubscribeTime": 1778788800000,
      "ResetTime": 1778806800000
    },
    "AFPDaily": {
      "Quota": 100.0,
      "Used": 22.5,
      "SubscribeTime": 1778716800000,
      "ResetTime": 1778803200000
    },
    "AFPWeekly": {
      "Quota": 500.0,
      "Used": 150.0,
      "SubscribeTime": 1778457600000,
      "ResetTime": 1779062400000
    },
    "AFPMonthly": {
      "Quota": 2000.0,
      "Used": 850.5,
      "SubscribeTime": 1777939200000,
      "ResetTime": 1780531200000
    }
  }
}"#;

    #[test]
    fn test_parse_sample_response() {
        let resp: AfpResponse = serde_json::from_str(SAMPLE).unwrap();
        assert!(resp.error.is_none());
        let result = resp.result.expect("result should exist");
        assert_eq!(result.plan_type, "Large");

        let five = result.five_hour.expect("five_hour should exist");
        assert_eq!(five.quota, 50.0);
        assert_eq!(five.used, 12.5);
        assert_eq!(five.subscribe_time, 1778788800000);
        assert_eq!(five.reset_time, 1778806800000);
        assert!((five.percent() - 25.0).abs() < 1e-6);
        assert!((five.remaining() - 37.5).abs() < 1e-6);
    }

    #[test]
    fn test_to_report() {
        let resp: AfpResponse = serde_json::from_str(SAMPLE).unwrap();
        let report = resp.into_report().unwrap();
        assert_eq!(report.plan_type, "Large");
        assert_eq!(report.windows.len(), 3);
        let five = report.windows.iter().find(|w| w.key == "five_hour").unwrap();
        assert_eq!(five.label, "5 小时");
        assert!((five.percent - 25.0).abs() < 1e-6);
    }

    #[test]
    fn test_parse_error_response() {
        let err_json = r#"{
          "ResponseMetadata": {"RequestId": "x"},
          "Error": {"Code": "InternalError", "Message": "boom"}
        }"#;
        let resp: AfpResponse = serde_json::from_str(err_json).unwrap();
        let result = resp.into_report();
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("InternalError"));
        assert!(msg.contains("boom"));
    }

    #[test]
    fn test_percent_clamped() {
        let w = Window { quota: 100.0, used: 150.0, subscribe_time: 0, reset_time: 0 };
        assert!((w.percent() - 100.0).abs() < 1e-6);
        assert!((w.remaining() - 0.0).abs() < 1e-6);
    }
}
