use serde::{Deserialize, Serialize};

use crate::cocli::error::{ColabError, Result};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretToken(String);

impl SecretToken {
    pub fn generate() -> Result<Self> {
        let mut bytes = [0u8; 32];
        getrandom::getrandom(&mut bytes)
            .map_err(|e| ColabError::config(format!("secure RNG failed: {e}")))?;
        Ok(Self(hex(&bytes)))
    }

    pub fn redacted(&self) -> &str {
        "<redacted>"
    }
}

impl std::fmt::Debug for SecretToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretToken(<redacted>)")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicRunId(String);

impl PublicRunId {
    pub fn generate(prefix: &str) -> Result<Self> {
        let mut bytes = [0u8; 6];
        getrandom::getrandom(&mut bytes)
            .map_err(|e| ColabError::config(format!("secure RNG failed: {e}")))?;
        Ok(Self(format!("{prefix}-{}", hex(&bytes))))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeterministicSeed(pub u64);

impl DeterministicSeed {
    pub fn next(self) -> u64 {
        // SplitMix64: deterministic plan IDs only, never for secrets.
        let mut z = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
}

pub fn secure_seed() -> Result<u64> {
    let mut bytes = [0u8; 8];
    getrandom::getrandom(&mut bytes)
        .map_err(|e| ColabError::config(format!("secure RNG failed: {e}")))?;
    Ok(u64::from_le_bytes(bytes))
}

fn hex(bytes: &[u8]) -> String {
    const TABLE: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(TABLE[(b >> 4) as usize] as char);
        out.push(TABLE[(b & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_seed_is_deterministic() {
        assert_eq!(
            DeterministicSeed(12345).next(),
            DeterministicSeed(12345).next()
        );
        assert_ne!(
            DeterministicSeed(12345).next(),
            DeterministicSeed(12346).next()
        );
    }

    #[test]
    fn secret_token_debug_is_redacted() {
        let token = SecretToken::generate().unwrap();
        assert_eq!(token.redacted(), "<redacted>");
        assert!(!format!("{token:?}").contains(&token.0));
    }

    #[test]
    fn secure_tokens_do_not_use_deterministic_path() {
        let a = SecretToken::generate().unwrap();
        let b = SecretToken::generate().unwrap();
        assert_ne!(a.0, b.0);
    }
}
