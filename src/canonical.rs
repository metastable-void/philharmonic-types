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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_value_sorts_top_level_keys() {
        let v: JsonValue = serde_json::from_str(r#"{"b":1,"a":2}"#).unwrap();
        let canon = CanonicalJson::from_value(&v).unwrap();
        assert_eq!(canon.as_bytes(), br#"{"a":2,"b":1}"#);
    }

    #[test]
    fn from_bytes_normalizes_whitespace_and_key_order() {
        let input = br#"{"z": 1,   "a":   2}"#;
        let canon = CanonicalJson::from_bytes(input).unwrap();
        assert_eq!(canon.as_bytes(), br#"{"a":2,"z":1}"#);
    }

    #[test]
    fn nested_objects_canonicalize_recursively() {
        let canon = CanonicalJson::from_bytes(br#"{"outer":{"z":1,"a":2}}"#).unwrap();
        assert_eq!(canon.as_bytes(), br#"{"outer":{"a":2,"z":1}}"#);
    }

    #[test]
    fn arrays_preserve_order_but_canonicalize_members() {
        let canon = CanonicalJson::from_bytes(br#"[{"b":1,"a":2},{"d":3,"c":4}]"#).unwrap();
        assert_eq!(canon.as_bytes(), br#"[{"a":2,"b":1},{"c":4,"d":3}]"#);
    }

    #[test]
    fn digest_is_stable_across_sender_key_orders() {
        let a = CanonicalJson::from_bytes(br#"{"b":1,"a":2}"#).unwrap();
        let b = CanonicalJson::from_bytes(br#"{"a":2,"b":1}"#).unwrap();
        assert_eq!(a.digest(), b.digest());
    }

    #[test]
    fn content_hash_matches_digest() {
        let canon = CanonicalJson::from_bytes(br#"{"a":1}"#).unwrap();
        assert_eq!(canon.content_hash().as_digest(), canon.digest());
    }

    #[test]
    fn eq_is_byte_equality_under_canonicalization() {
        let a = CanonicalJson::from_bytes(br#"{"x":1}"#).unwrap();
        let b = CanonicalJson::from_bytes(br#"{  "x":   1   }"#).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn from_serializable_round_trips_typed_value() {
        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        struct Shape {
            z: i32,
            a: String,
        }
        let original = Shape {
            z: 1,
            a: "hello".to_string(),
        };
        let canon = CanonicalJson::from_serializable(&original).unwrap();
        // Keys must come out alphabetized: a then z.
        assert_eq!(canon.as_bytes(), br#"{"a":"hello","z":1}"#);
        let back: Shape = canon.to_deserializable().unwrap();
        assert_eq!(back, original);
    }

    #[test]
    fn from_bytes_rejects_non_json() {
        let result = CanonicalJson::from_bytes(b"not json");
        assert!(matches!(result, Err(CanonError::Json(_))));
    }

    #[test]
    fn from_bytes_rejects_trailing_garbage() {
        let result = CanonicalJson::from_bytes(br#"{"a":1} extra"#);
        assert!(result.is_err());
    }

    #[test]
    fn serialize_emits_json_value_not_byte_array() {
        let canon = CanonicalJson::from_bytes(br#"{"a":1}"#).unwrap();
        let as_json = serde_json::to_string(&canon).unwrap();
        // Must appear as `{"a":1}`, not `[123, 34, 97, ...]`.
        assert_eq!(as_json, r#"{"a":1}"#);
    }

    #[test]
    fn deserialize_canonicalizes_sender_order() {
        let json = r#"{"z":1,"a":2}"#;
        let canon: CanonicalJson = serde_json::from_str(json).unwrap();
        assert_eq!(canon.as_bytes(), br#"{"a":2,"z":1}"#);
    }

    #[test]
    fn content_trait_round_trips_through_bytes() {
        let canon = CanonicalJson::from_bytes(br#"{"a":1}"#).unwrap();
        let bytes = canon.to_content_bytes();
        let back = CanonicalJson::from_content_bytes(bytes.as_ref()).unwrap();
        assert_eq!(canon, back);
    }

    #[test]
    fn debug_format_renders_as_json_text() {
        let canon = CanonicalJson::from_bytes(br#"{"a":1}"#).unwrap();
        assert_eq!(format!("{canon:?}"), r#"CanonicalJson({"a":1})"#);
    }

    #[test]
    fn into_bytes_consumes_and_returns_canonical_bytes() {
        let canon = CanonicalJson::from_bytes(br#"{"a":1}"#).unwrap();
        assert_eq!(canon.into_bytes(), br#"{"a":1}"#);
    }

    #[test]
    fn hash_impl_treats_equal_values_as_equal_keys() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(CanonicalJson::from_bytes(br#"{"b":1,"a":2}"#).unwrap());
        assert!(set.contains(&CanonicalJson::from_bytes(br#"{"a":2,"b":1}"#).unwrap()));
    }

    #[test]
    fn unicode_keys_sort_by_codepoint_order() {
        // Keys "a" (U+0061), "z" (U+007A), "é" (U+00E9).
        // Expected JCS order: a, z, é.
        let canon = CanonicalJson::from_bytes("{\"é\":3,\"z\":2,\"a\":1}".as_bytes()).unwrap();
        let text = std::str::from_utf8(canon.as_bytes()).unwrap();
        let a_pos = text.find("\"a\"").unwrap();
        let z_pos = text.find("\"z\"").unwrap();
        let e_pos = text.find('é').unwrap();
        assert!(a_pos < z_pos);
        assert!(z_pos < e_pos);
    }
}
