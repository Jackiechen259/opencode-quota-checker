use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

/// 火山引擎 Signature V4 签名结果
#[derive(Debug, Clone)]
pub struct SignedRequest {
    pub authorization: String,
    pub x_date: String,
    pub x_content_sha256: String,
}

/// 对齐 Python SDK `SignerV4.sign` 的实现。
///
/// `query` 按 key 字典序、URL 编码(safe: `-_.~`)排列。
/// `signed_headers` 取 `content-type`/`content-md5`/`host`/`x-*` 头(全小写 key)。
/// 密钥派生链: HMAC(SK,date) -> region -> service -> "request"，无前缀。
#[allow(clippy::too_many_arguments)]
pub fn sign_v4(
    method: &str,
    path: &str,
    host: &str,
    query: &[(&str, &str)],
    body: &[u8],
    ak: &str,
    sk: &str,
    region: &str,
    service: &str,
    content_type: &str,
    format_date: &str,
) -> SignedRequest {
    let body_hash = hex::encode(Sha256::digest(body));

    // 构造 signed headers: SDK 取 Content-Type / Content-Md5 / Host / X-* 头
    // 这里固定 4 个头: content-type, host, x-content-sha256, x-date
    let mut signed_headers: Vec<(&str, &str)> = vec![
        ("content-type", content_type),
        ("host", host),
        ("x-content-sha256", &body_hash),
        ("x-date", format_date),
    ];
    signed_headers.sort_by_key(|(k, _)| *k);

    // signed_str: 每行 "key:value\n", 已按 key 排序
    let signed_str: String = signed_headers
        .iter()
        .map(|(k, v)| format!("{}:{}\n", k, v))
        .collect();

    let signed_headers_string: String = signed_headers
        .iter()
        .map(|(k, _)| *k)
        .collect::<Vec<_>>()
        .join(";");

    let canonical_query = canonical_query(query);

    let canonical_request = [
        method.to_string(),
        path.to_string(),
        canonical_query,
        signed_str,
        signed_headers_string.clone(),
        body_hash.clone(),
    ]
    .join("\n");

    let credential_scope = format!("{}/{}/{}/request", &format_date[..8], region, service);

    let string_to_sign = [
        "HMAC-SHA256".to_string(),
        format_date.to_string(),
        credential_scope.clone(),
        hex::encode(Sha256::digest(canonical_request.as_bytes())),
    ]
    .join("\n");

    let signing_key = get_signing_secret_key_v4(sk, &format_date[..8], region, service);

    let signature = hex::encode(
        HmacSha256::new_from_slice(&signing_key)
            .expect("HMAC key length is always valid")
            .chain_update(string_to_sign.as_bytes())
            .finalize()
            .into_bytes(),
    );

    let authorization = format!(
        "HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
        ak, credential_scope, signed_headers_string, signature
    );

    SignedRequest {
        authorization,
        x_date: format_date.to_string(),
        x_content_sha256: body_hash,
    }
}

/// 对齐 Python SDK `SignerV4.canonical_query`
/// 每个 key/value 用 quote(safe='-_.~') 编码,按 key 排序, & 连接
fn canonical_query(query: &[(&str, &str)]) -> String {
    let mut pairs: Vec<(String, String)> = query
        .iter()
        .map(|(k, v)| {
            (
                urlencoding::encode(k).to_string(),
                urlencoding::encode(v).to_string(),
            )
        })
        .collect();
    pairs.sort();
    pairs
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("&")
}

/// 对齐 Python SDK `SignerV4.get_signing_secret_key_v4`
/// kdate = HMAC(SK, date); kregion = HMAC(kdate, region); kservice = HMAC(kregion, service); ksigning = HMAC(kservice, "request")
fn get_signing_secret_key_v4(sk: &str, date: &str, region: &str, service: &str) -> Vec<u8> {
    let kdate = hmac_sha256(sk.as_bytes(), date);
    let kregion = hmac_sha256(&kdate, region);
    let kservice = hmac_sha256(&kregion, service);
    hmac_sha256(&kservice, "request")
}

fn hmac_sha256(key: &[u8], msg: &str) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key length is always valid");
    mac.update(msg.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_AK: &str = "AKLTTestAccessKey001";
    const TEST_SK: &str = "W1 secret key for testing 001";
    const TEST_DATE: &str = "20260511T034034Z";
    const TEST_REGION: &str = "cn-beijing";
    const TEST_SERVICE: &str = "ark";
    const TEST_HOST: &str = "ark.cn-beijing.volces.com";
    const TEST_CONTENT_TYPE: &str = "application/json; charset=UTF-8";

    #[test]
    fn test_body_hash_of_empty_json() {
        let body = b"{}";
        let hash = hex::encode(Sha256::digest(body));
        assert_eq!(
            hash,
            "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a"
        );
    }

    #[test]
    fn test_body_hash_of_empty_string() {
        let body = b"";
        let hash = hex::encode(Sha256::digest(body));
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_canonical_query_basic() {
        let query = vec![("Action", "GetAFPUsage"), ("Version", "2024-01-01")];
        let result = canonical_query(&query);
        assert_eq!(result, "Action=GetAFPUsage&Version=2024-01-01");
    }

    #[test]
    fn test_canonical_query_sorts_by_key() {
        let query = vec![("Version", "2024-01-01"), ("Action", "GetAFPUsage")];
        let result = canonical_query(&query);
        assert_eq!(result, "Action=GetAFPUsage&Version=2024-01-01");
    }

    #[test]
    fn test_canonical_query_encodes_special_chars() {
        let query = vec![("K", "a b/c")];
        let result = canonical_query(&query);
        // urlencoding encodes space as %20, / as %2F
        assert_eq!(result, "K=a%20b%2Fc");
    }

    #[test]
    fn test_signing_key_derivation() {
        let key = get_signing_secret_key_v4("sk123", "20260711", "cn-beijing", "ark");
        assert_eq!(
            hex::encode(key),
            "8d2c244d0a0fec86709100869e279cd76c6a33c61ac316b35f5a347667383e95"
        );
    }

    #[test]
    fn test_sign_v4_produces_valid_structure() {
        let body = b"{}";
        let query = vec![("Action", "GetAFPUsage"), ("Version", "2024-01-01")];
        let result = sign_v4(
            "POST",
            "/",
            TEST_HOST,
            &query,
            body,
            TEST_AK,
            TEST_SK,
            TEST_REGION,
            TEST_SERVICE,
            TEST_CONTENT_TYPE,
            TEST_DATE,
        );

        assert!(result.authorization.starts_with("HMAC-SHA256 Credential="));
        assert!(result
            .authorization
            .contains("SignedHeaders=content-type;host;x-content-sha256;x-date"));
        assert!(result.authorization.contains("Signature="));
        assert_eq!(result.x_date, TEST_DATE);
        assert_eq!(
            result.x_content_sha256,
            "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a"
        );
    }

    #[test]
    fn test_sign_v4_authorization_credential_scope() {
        let body = b"{}";
        let query = vec![("Action", "GetAFPUsage"), ("Version", "2024-01-01")];
        let result = sign_v4(
            "POST",
            "/",
            TEST_HOST,
            &query,
            body,
            TEST_AK,
            TEST_SK,
            TEST_REGION,
            TEST_SERVICE,
            TEST_CONTENT_TYPE,
            TEST_DATE,
        );
        let expected_credential =
            format!("{}/{}/{}/request", "20260511", TEST_REGION, TEST_SERVICE);
        assert!(
            result
                .authorization
                .contains(&format!("Credential={}/{}", TEST_AK, expected_credential)),
            "credential scope mismatch, got: {}",
            result.authorization
        );
    }

    #[test]
    fn test_sign_v4_deterministic() {
        let body = b"{}";
        let query = vec![("Action", "GetAFPUsage"), ("Version", "2024-01-01")];
        let args = (
            "POST",
            "/",
            TEST_HOST,
            query.as_slice(),
            body.as_slice(),
            TEST_AK,
            TEST_SK,
            TEST_REGION,
            TEST_SERVICE,
            TEST_CONTENT_TYPE,
            TEST_DATE,
        );
        let r1 = sign_v4(
            args.0, args.1, args.2, args.3, args.4, args.5, args.6, args.7, args.8, args.9, args.10,
        );
        let r2 = sign_v4(
            args.0, args.1, args.2, args.3, args.4, args.5, args.6, args.7, args.8, args.9, args.10,
        );
        assert_eq!(
            r1.authorization, r2.authorization,
            "signing must be deterministic for identical inputs"
        );
    }
}
