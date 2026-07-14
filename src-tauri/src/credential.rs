use anyhow::{anyhow, Result};

const SERVICE: &str = "volc-status";
const ACCOUNT: &str = "volcengine-ak-sk";
/// AK 与 SK 之间的分隔符,使用 \x00 避免与正常内容冲突
const SEP: &str = "\x00";

/// 将 AK/SK 存入系统密钥环
pub fn save(ak: &str, sk: &str) -> Result<()> {
    let entry = keyring::Entry::new(SERVICE, ACCOUNT)?;
    let combined = format!("{}{}{}", ak, SEP, sk);
    entry.set_password(&combined)?;
    Ok(())
}

/// 从系统密钥环读取 AK/SK
pub fn load() -> Result<(String, String)> {
    let entry = keyring::Entry::new(SERVICE, ACCOUNT)?;
    let combined = entry.get_password()?;
    match combined.split_once(SEP) {
        Some((ak, sk)) if !ak.is_empty() && !sk.is_empty() => Ok((ak.to_string(), sk.to_string())),
        _ => Err(anyhow!("凭证格式无效")),
    }
}

/// 是否已配置凭证
pub fn has() -> bool {
    load().is_ok()
}

/// 删除凭证
pub fn clear() -> Result<()> {
    let entry = keyring::Entry::new(SERVICE, ACCOUNT)?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(anyhow!("删除凭证失败: {}", e)),
    }
}
