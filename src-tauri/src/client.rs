use crate::models::AfpResponse;
use crate::signing::sign_v4;
use anyhow::{anyhow, Result};

const HOST: &str = "open.volcengineapi.com";
const PATH: &str = "/";
const REGION: &str = "cn-beijing";
const SERVICE: &str = "ark";
const CONTENT_TYPE: &str = "application/json; charset=UTF-8";
const ACTION: &str = "GetAFPUsage";
const VERSION: &str = "2024-01-01";
const TIMEOUT_SECS: u64 = 15;

/// 发送签名或带 Token 的 GetAFPUsage 请求,返回原始响应 body
pub async fn fetch_afp_usage(ak: &str, sk: &str) -> Result<String> {
    let url = format!("https://{}{}", HOST, PATH);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(TIMEOUT_SECS))
        .build()?;

    let body = b"{}";
    let query: [(&str, &str); 2] = [("Action", ACTION), ("Version", VERSION)];

    let mut req = client
        .post(&url)
        .query(&query)
        .header("Host", HOST)
        .header("Content-Type", CONTENT_TYPE)
        .body(body.to_vec());

    let format_date = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let signed = sign_v4(
        "POST",
        PATH,
        HOST,
        &query,
        body,
        ak,
        sk,
        REGION,
        SERVICE,
        CONTENT_TYPE,
        &format_date,
    );
    req = req
        .header("X-Date", &signed.x_date)
        .header("X-Content-Sha256", &signed.x_content_sha256)
        .header("Authorization", &signed.authorization);

    let resp = req.send().await?;

    let status = resp.status();
    let text = resp.text().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "HTTP {}: {}",
            status.as_u16(),
            truncate(&text, 500)
        ));
    }

    Ok(text)
}

/// 解析响应并转换为 UsageReport
pub async fn fetch_report(ak: &str, sk: &str) -> Result<crate::models::UsageReport> {
    let raw = fetch_afp_usage(ak, sk).await?;
    let resp: AfpResponse = serde_json::from_str(&raw)
        .map_err(|e| anyhow!("解析响应失败: {} | 原始: {}", e, truncate(&raw, 500)))?;
    resp.into_report()
        .map_err(|e| anyhow!("接口返回错误: {}", e))
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...(截断)", &s[..max])
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_short() {
        assert_eq!(truncate("abc", 10), "abc");
    }

    #[test]
    fn test_truncate_long() {
        let long = "a".repeat(600);
        let t = truncate(&long, 500);
        assert!(t.ends_with("...(截断)"));
        assert!(t.len() < 600);
    }
}
