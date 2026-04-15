use crate::{Content, ContentDecodeError, ContentHash, Sha256};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::DeserializeOwned};

pub use serde_json::Map as JsonMap;
pub use serde_json::Value as JsonValue;

use std::borrow::Cow;

/// Canonical JSON bytes per RFC 8785 (JCS).
///
/// The invariant is that the bytes inside are JCS-canonical: keys sorted
/// at every level of nesting, numbers formatted per ECMA-262, strings
/// escaped per RFC 8259, no insignificant whitespace. Values of this type
/// can be hashed directly for content-addressing without further
/// canonicalization.
///
/// All public constructors enforce the invariant. Deserialization goes
/// through canonicalization, so a `CanonicalJson` received from the wire
/// is always canonical regardless of how the sender serialized it.
#[derive(Clone)]
pub struct CanonicalJson(Vec<u8>);

impl CanonicalJson {
    /// Canonicalize a `JsonValue` via JCS and store the resulting bytes.
    pub fn from_value(v: &JsonValue) -> Result<Self, CanonError> {
        let bytes = serde_jcs::to_vec(v)?;
        Ok(Self(bytes))
    }

    /// Parse arbitrary JSON bytes and re-emit them in canonical form.
    ///
    /// The input bytes need not be canonical; the output always is.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CanonError> {
        let v: JsonValue = serde_json::from_slice(bytes)?;
        Self::from_value(&v)
    }

    /// The canonical bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Consume self and return the canonical bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }

    /// The typed content hash of these canonical bytes.
    ///
    /// Because the bytes are canonical by construction, the hash is
    /// stable: two `CanonicalJson` values that represent the same JSON
    /// value (same keys, same values, any original key order) hash to
    /// the same digest.
    pub fn content_hash(&self) -> ContentHash<Self> {
        ContentHash::of_bytes_unchecked(&self.0)
    }

    /// The raw digest of these canonical bytes.
    ///
    /// Convenience for when you want the untyped hash; equivalent to
    /// `self.content_hash().as_digest()`.
    pub fn digest(&self) -> Sha256 {
        Sha256::of(&self.0)
    }

    /// Serialize a typed value to canonical JSON.
    ///
    /// Equivalent to `from_value(&serde_json::to_value(v)?)` but expressed
    /// as one call. Useful when a typed Rust value needs to become
    /// content-addressable.
    pub fn from_serializable<T: Serialize>(v: &T) -> Result<Self, CanonError> {
        let value = serde_json::to_value(v)?;
        Self::from_value(&value)
    }

    /// Deserialize the canonical JSON into a typed value.
    ///
    /// The bytes are guaranteed valid JSON by construction, so this only
    /// fails if the JSON shape doesn't match `T`'s expected shape.
    pub fn to_deserializable<T: DeserializeOwned>(&self) -> Result<T, CanonError> {
        serde_json::from_slice(&self.0).map_err(CanonError::from)
    }
}

impl std::fmt::Debug for CanonicalJson {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Debug as the JSON it represents, not as raw bytes.
        // The bytes are guaranteed valid JSON by construction, so this
        // parse cannot fail in practice.
        match std::str::from_utf8(&self.0) {
            Ok(s) => write!(f, "CanonicalJson({})", s),
            Err(_) => write!(f, "CanonicalJson(<invalid utf-8, {} bytes>)", self.0.len()),
        }
    }
}

impl PartialEq for CanonicalJson {
    fn eq(&self, other: &Self) -> bool {
        // Because both sides are canonical by construction, byte equality
        // is value equality. No need to reparse.
        self.0 == other.0
    }
}

impl Eq for CanonicalJson {}

impl std::hash::Hash for CanonicalJson {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

/// Serialize as the JSON value the bytes represent, not as a byte array.
///
/// A `CanonicalJson` containing `{"foo":1}` serializes to the JSON value
/// `{"foo":1}`, not to an array of byte numbers. This is the right wire
/// format for any JSON-shaped serializer; other serializers (CBOR, etc.)
/// will emit their own natural encoding of the parsed value.
impl Serialize for CanonicalJson {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        // The bytes are valid JSON by construction.
        let value: JsonValue =
            serde_json::from_slice(&self.0).map_err(serde::ser::Error::custom)?;
        value.serialize(s)
    }
}

/// Deserialize from any JSON value, then canonicalize.
///
/// Input JSON can be in any key order; the resulting `CanonicalJson`
/// invariant (canonical bytes) is established during deserialization.
/// This means `CanonicalJson` received over the wire is always canonical
/// regardless of the sender's serialization choices.
impl<'de> Deserialize<'de> for CanonicalJson {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let value = JsonValue::deserialize(d)?;
        Self::from_value(&value).map_err(serde::de::Error::custom)
    }
}

/// `CanonicalJson` is content-addressable: its canonical bytes are its
/// content, and decoding runs JCS canonicalization on arbitrary JSON
/// input to re-establish the invariant.
impl Content for CanonicalJson {
    fn to_content_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Borrowed(&self.0)
    }

    fn from_content_bytes(bytes: &[u8]) -> Result<Self, ContentDecodeError> {
        Self::from_bytes(bytes).map_err(ContentDecodeError::from)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CanonError {
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
}

impl From<CanonError> for ContentDecodeError {
    fn from(e: CanonError) -> Self {
        match e {
            CanonError::Json(json_err) => ContentDecodeError::Json(json_err),
        }
    }
}
