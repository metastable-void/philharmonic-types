use crate::Sha256;

use serde::{Deserialize, Serialize, Serializer, Deserializer};

use serde_jcs as jcs;

#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct CanonicalJson(Vec<u8>);

impl CanonicalJson {
    /// Canonicalize a serde_json::Value via JCS (RFC 8785) and store the bytes.
    pub fn from_value(v: &serde_json::Value) -> Result<Self, CanonError> {
        let bytes = jcs::to_vec(v)?;
        Ok(Self(bytes))
    }

    /// Parse arbitrary JSON bytes and re-emit canonically.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CanonError> {
        let v: serde_json::Value = serde_json::from_slice(bytes)?;
        Self::from_value(&v)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }

    pub fn hash(&self) -> Sha256 {
        Sha256::of(&self.0)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CanonError {
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("canonicalization failed: {0}")]
    Canon(String),
}

impl Serialize for CanonicalJson {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        // Parse the canonical bytes back into a Value, then serialize that.
        // The bytes are guaranteed valid JSON by construction.
        let v: serde_json::Value = serde_json::from_slice(&self.0)
            .map_err(serde::ser::Error::custom)?;
        v.serialize(s)
    }
}

impl<'de> Deserialize<'de> for CanonicalJson {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        // Deserialize as a Value, then canonicalize through from_value,
        // which enforces the invariant.
        let v = serde_json::Value::deserialize(d)?;
        Self::from_value(&v).map_err(serde::de::Error::custom)
    }
}
