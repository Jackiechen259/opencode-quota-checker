use crate::VolcError;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

/// Headers produced by Volcano Engine Signature V4.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedRequest {
    /// Authorization header value.
    pub authorization: String,
    /// `X-Date` header value.
    pub x_date: String,
    /// SHA-256 body digest.
    pub x_content_sha256: String,
}

/// Signs a request using Volcano Engine Signature V4.
#[allow(clippy::too_many_arguments)]
pub fn sign_v4(
    method: &str,
    path: &str,
    host: &str,
    query: &[(&str, &str)],
    body: &[u8],
    access_key: &str,
    secret_key: &str,
    region: &str,
    service: &str,
    content_type: &str,
    format_date: &str,
) -> Result<SignedRequest, VolcError> {
    let date = format_date
        .get(..8)
        .filter(|date| date.chars().all(|character| character.is_ascii_digit()))
        .ok_or_else(|| VolcError::Signing("X-Date must start with YYYYMMDD".to_owned()))?;
    let body_hash = hex::encode(Sha256::digest(body));
    let mut signed_headers = [
        ("content-type", content_type),
        ("host", host),
        ("x-content-sha256", body_hash.as_str()),
        ("x-date", format_date),
    ];
    signed_headers.sort_by_key(|(key, _)| *key);

    let signed_values = signed_headers
        .iter()
        .map(|(key, value)| format!("{key}:{value}\n"))
        .collect::<String>();
    let signed_names = signed_headers
        .iter()
        .map(|(key, _)| *key)
        .collect::<Vec<_>>()
        .join(";");
    let canonical_request = [
        method.to_owned(),
        path.to_owned(),
        canonical_query(query),
        signed_values,
        signed_names.clone(),
        body_hash.clone(),
    ]
    .join("\n");
    let credential_scope = format!("{date}/{region}/{service}/request");
    let string_to_sign = [
        "HMAC-SHA256".to_owned(),
        format_date.to_owned(),
        credential_scope.clone(),
        hex::encode(Sha256::digest(canonical_request.as_bytes())),
    ]
    .join("\n");
    let signing_key = signing_key(secret_key, date, region, service)?;
    let signature = hex::encode(hmac_sha256(&signing_key, string_to_sign.as_bytes())?);
    let authorization = format!(
        "HMAC-SHA256 Credential={access_key}/{credential_scope}, SignedHeaders={signed_names}, Signature={signature}"
    );

    Ok(SignedRequest {
        authorization,
        x_date: format_date.to_owned(),
        x_content_sha256: body_hash,
    })
}

fn canonical_query(query: &[(&str, &str)]) -> String {
    let mut pairs = query
        .iter()
        .map(|(key, value)| {
            (
                urlencoding::encode(key).into_owned(),
                urlencoding::encode(value).into_owned(),
            )
        })
        .collect::<Vec<_>>();
    pairs.sort();
    pairs
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&")
}

fn signing_key(
    secret_key: &str,
    date: &str,
    region: &str,
    service: &str,
) -> Result<Vec<u8>, VolcError> {
    let date_key = hmac_sha256(secret_key.as_bytes(), date.as_bytes())?;
    let region_key = hmac_sha256(&date_key, region.as_bytes())?;
    let service_key = hmac_sha256(&region_key, service.as_bytes())?;
    hmac_sha256(&service_key, b"request")
}

fn hmac_sha256(key: &[u8], message: &[u8]) -> Result<Vec<u8>, VolcError> {
    let mut mac = HmacSha256::new_from_slice(key)
        .map_err(|_| VolcError::Signing("invalid HMAC key".to_owned()))?;
    mac.update(message);
    Ok(mac.finalize().into_bytes().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACCESS_KEY: &str = "AKLTTestAccessKey001";
    const SECRET_KEY: &str = "W1 secret key for testing 001";

    #[test]
    fn canonical_query_sorts_and_encodes() {
        assert_eq!(
            canonical_query(&[("Version", "2024-01-01"), ("A key", "a b/c")]),
            "A%20key=a%20b%2Fc&Version=2024-01-01"
        );
    }

    #[test]
    fn signing_key_matches_fixed_vector() {
        let key = signing_key("sk123", "20260711", "cn-beijing", "ark")
            .expect("test signing key is valid");
        assert_eq!(
            hex::encode(key),
            "8d2c244d0a0fec86709100869e279cd76c6a33c61ac316b35f5a347667383e95"
        );
    }

    #[test]
    fn signature_is_deterministic_and_structured() {
        let signed = sign_v4(
            "POST",
            "/",
            "open.volcengineapi.com",
            &[("Action", "GetAFPUsage"), ("Version", "2024-01-01")],
            b"{}",
            ACCESS_KEY,
            SECRET_KEY,
            "cn-beijing",
            "ark",
            "application/json; charset=UTF-8",
            "20260511T034034Z",
        )
        .expect("fixed signing input is valid");

        assert_eq!(
            signed.x_content_sha256,
            "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a"
        );
        assert!(signed
            .authorization
            .contains("Credential=AKLTTestAccessKey001/20260511/cn-beijing/ark/request"));
        assert!(signed
            .authorization
            .contains("SignedHeaders=content-type;host;x-content-sha256;x-date"));
    }

    #[test]
    fn short_date_is_rejected_without_panicking() {
        let result = sign_v4(
            "POST",
            "/",
            "example.com",
            &[],
            b"",
            ACCESS_KEY,
            SECRET_KEY,
            "cn-beijing",
            "ark",
            "application/json",
            "bad",
        );
        assert!(result.is_err());
    }
}
