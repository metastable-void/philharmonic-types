
use crate::Sha256;

use serde::{Deserialize, Serialize};

use serde_jcs as jcs;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
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

    pub fn as_bytes(&self) -> &[u8] { &self.0 }
    pub fn into_bytes(self) -> Vec<u8> { self.0 }

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
