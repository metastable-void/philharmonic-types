use crate::Sha256;

use serde::{Deserialize, Serialize};

use std::{borrow::Cow, fmt, marker::PhantomData};

/// A hash function suitable for content-addressing.
///
/// Implementors are both the function designator (used as a type parameter
/// to `ContentHash`) and the output type produced by hashing.
/// `HashFunction::Output = Self` is the standard shape; see the `Sha256`
/// impl in `crate::sha256` for the canonical example.
pub trait HashFunction: Copy + Eq + std::hash::Hash + Send + Sync + 'static {
    /// The digest type produced by this hash function.
    type Output: Copy + Eq + std::hash::Hash + Send + Sync + 'static;

    /// Hash the given bytes into an output digest.
    fn digest(bytes: &[u8]) -> Self::Output;
}

/// A type whose values can be content-addressed.
///
/// Implementors declare how to encode themselves into bytes and how to
/// decode bytes back into a value. Encoding must be deterministic: the same
/// value must always produce the same bytes, so that content addresses
/// remain stable.
pub trait Content: Sized {
    /// Encode this value to its canonical byte representation.
    ///
    /// Types that already hold their canonical bytes should return
    /// `Cow::Borrowed`; types that compute them on demand should return
    /// `Cow::Owned`.
    fn to_content_bytes(&self) -> Cow<'_, [u8]>;

    /// Decode a value from bytes. Returns an error if the bytes are not
    /// a valid encoding of `Self`.
    fn from_content_bytes(bytes: &[u8]) -> Result<Self, ContentDecodeError>;
}

/// A hash of a specific content type, computed with a specific hash function.
///
/// Wraps a raw digest with a phantom type parameter that records what the
/// digest is a hash *of*. This lets APIs enforce at compile time that a
/// `ContentHash<CanonicalJson>` is not accidentally passed where a
/// `ContentHash<JsScript>` is expected, even though the underlying bytes
/// have identical shape.
///
/// The `F` parameter defaults to `Sha256`, which is the canonical choice
/// for this system; code that doesn't care about the hash function can
/// write `ContentHash<T>` and get SHA-256 implicitly.
#[repr(transparent)]
pub struct ContentHash<T: Content, F: HashFunction = Sha256> {
    digest: F::Output,
    _phantom: PhantomData<fn() -> T>,
}

impl<T: Content, F: HashFunction> ContentHash<T, F> {
    /// Compute the hash of a content value.
    pub fn of(content: &T) -> Self
    where
        T: Content,
    {
        let bytes = content.to_content_bytes();
        Self {
            digest: F::digest(&bytes),
            _phantom: PhantomData,
        }
    }

    /// Compute the hash of raw bytes, tagging it as a hash of `T`.
    ///
    /// The caller asserts that the bytes are a valid encoding of `T`.
    /// Used when bytes are already in hand and re-encoding would be wasteful.
    pub fn of_bytes_unchecked(bytes: &[u8]) -> Self {
        Self {
            digest: F::digest(bytes),
            _phantom: PhantomData,
        }
    }

    /// Wrap a raw digest as a typed content hash.
    ///
    /// The caller asserts that the digest was computed from bytes that
    /// validly encode `T`. Used when reading typed hashes back from
    /// trusted storage.
    pub fn from_digest_unchecked(digest: F::Output) -> Self {
        Self {
            digest,
            _phantom: PhantomData,
        }
    }

    /// The underlying digest, stripped of its content-type tag.
    pub fn as_digest(&self) -> F::Output {
        self.digest
    }
}

// Manual impls: can't derive Clone/Copy/Eq/Hash directly because
// PhantomData<fn() -> T> doesn't constrain them, and we want the impls
// regardless of what T is.
impl<T: Content, F: HashFunction> Clone for ContentHash<T, F> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: Content, F: HashFunction> Copy for ContentHash<T, F> {}

impl<T: Content, F: HashFunction> PartialEq for ContentHash<T, F> {
    fn eq(&self, other: &Self) -> bool {
        self.digest == other.digest
    }
}
impl<T: Content, F: HashFunction> Eq for ContentHash<T, F> {}

impl<T: Content, F: HashFunction> std::hash::Hash for ContentHash<T, F> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.digest.hash(state);
    }
}

impl<T: Content, F: HashFunction> fmt::Debug for ContentHash<T, F>
where
    F::Output: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ContentHash").field(&self.digest).finish()
    }
}

// Serialization is transparent over the digest: a ContentHash<T, F> on the
// wire looks exactly like an F::Output, because the T tag is compile-time
// information only.
impl<T: Content, F: HashFunction> Serialize for ContentHash<T, F>
where
    F::Output: Serialize,
{
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.digest.serialize(s)
    }
}

impl<'de, T: Content, F: HashFunction> Deserialize<'de> for ContentHash<T, F>
where
    F::Output: Deserialize<'de>,
{
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let digest = F::Output::deserialize(d)?;
        Ok(Self::from_digest_unchecked(digest))
    }
}

/// An untyped content value: bytes and their hash, together.
///
/// This is the storage-boundary form of a content-addressed value. Callers
/// who have a typed `T: Content` in hand should generally hash and store
/// `T` directly via the content-store API; `ContentValue` is useful for
/// pipeline code that shuttles content between producer and consumer
/// without caring what type it is.
///
/// Invariant: `self.hash() == Sha256::of(self.bytes())`. Enforced by
/// construction via `ContentValue::new`; the invariant is not checked on
/// deserialization (the assumption is that deserialized values come from
/// a trusted store that verified them on write).
#[derive(Clone, Debug)]
pub struct ContentValue {
    digest: Sha256,
    bytes: Vec<u8>,
}

impl ContentValue {
    /// Construct a `ContentValue` from raw bytes, computing the hash.
    pub fn new(bytes: Vec<u8>) -> Self {
        let hash = Sha256::of(&bytes);
        Self {
            digest: hash,
            bytes,
        }
    }

    /// Construct a `ContentValue` from bytes and a pre-computed hash,
    /// without verifying that the hash matches.
    ///
    /// Used when reading from a trusted content store that has already
    /// verified the invariant on write.
    pub fn from_parts_unchecked(hash: Sha256, bytes: Vec<u8>) -> Self {
        Self {
            digest: hash,
            bytes,
        }
    }

    pub fn digest(&self) -> Sha256 {
        self.digest
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Attempt to decode this content value as a specific `Content` type.
    pub fn decode<T: Content>(&self) -> Result<T, ContentDecodeError> {
        T::from_content_bytes(&self.bytes)
    }
}

impl<T: Content> From<&T> for ContentValue {
    fn from(value: &T) -> Self {
        Self::new(value.to_content_bytes().into_owned())
    }
}

/// `ContentValue` is trivially `Content`: its encoding is its bytes,
/// its decoding is wrapping bytes in a new `ContentValue`.
///
/// This impl is what makes `ContentHash<ContentValue>` meaningful as the
/// default untyped content hash — a hash of "bytes, not yet decoded as
/// anything in particular."
impl Content for ContentValue {
    fn to_content_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Borrowed(&self.bytes)
    }

    fn from_content_bytes(bytes: &[u8]) -> Result<Self, ContentDecodeError> {
        Ok(Self::new(bytes.to_vec()))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ContentDecodeError {
    #[error("content bytes are not valid UTF-8: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("content bytes are not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("content decode failed: {0}")]
    Custom(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal Content impl so we can exercise the trait surface without
    // pulling in crate::canonical (which brings JSON semantics).
    #[derive(Debug, PartialEq, Eq)]
    struct BytesBag(Vec<u8>);

    impl Content for BytesBag {
        fn to_content_bytes(&self) -> Cow<'_, [u8]> {
            Cow::Borrowed(&self.0)
        }

        fn from_content_bytes(bytes: &[u8]) -> Result<Self, ContentDecodeError> {
            Ok(Self(bytes.to_vec()))
        }
    }

    #[test]
    fn content_value_new_computes_sha256() {
        let val = ContentValue::new(b"hello".to_vec());
        assert_eq!(val.digest(), Sha256::of(b"hello"));
        assert_eq!(val.bytes(), b"hello");
    }

    #[test]
    fn content_value_from_parts_unchecked_skips_verification() {
        // The point of `from_parts_unchecked` is that it trusts the caller.
        // We prove that by feeding it a bogus hash.
        let fake = Sha256::of(b"lies");
        let val = ContentValue::from_parts_unchecked(fake, b"truth".to_vec());
        assert_eq!(val.digest(), fake);
        assert_ne!(val.digest(), Sha256::of(val.bytes()));
    }

    #[test]
    fn content_value_into_bytes_consumes() {
        let val = ContentValue::new(b"payload".to_vec());
        assert_eq!(val.into_bytes(), b"payload");
    }

    #[test]
    fn content_value_decode_round_trips() {
        let original = BytesBag(b"payload".to_vec());
        let val: ContentValue = (&original).into();
        let back: BytesBag = val.decode().unwrap();
        assert_eq!(back, original);
    }

    #[test]
    fn content_value_content_impl_wraps_new() {
        let raw = b"hello".to_vec();
        let via_trait = <ContentValue as Content>::from_content_bytes(&raw).unwrap();
        let direct = ContentValue::new(raw.clone());
        assert_eq!(via_trait.bytes(), direct.bytes());
        assert_eq!(via_trait.digest(), direct.digest());
    }

    #[test]
    fn content_hash_of_matches_of_bytes_unchecked_for_same_bytes() {
        let c = BytesBag(b"payload".to_vec());
        let from_content = ContentHash::<BytesBag>::of(&c);
        let from_bytes = ContentHash::<BytesBag>::of_bytes_unchecked(b"payload");
        assert_eq!(from_content, from_bytes);
    }

    #[test]
    fn content_hash_as_digest_unwraps_phantom_tag() {
        let c = BytesBag(b"payload".to_vec());
        let hash = ContentHash::<BytesBag>::of(&c);
        assert_eq!(hash.as_digest(), Sha256::of(b"payload"));
    }

    #[test]
    fn content_hash_from_digest_unchecked_wraps_arbitrary_digest() {
        let digest = Sha256::of(b"anything");
        let wrapped = ContentHash::<BytesBag>::from_digest_unchecked(digest);
        assert_eq!(wrapped.as_digest(), digest);
    }

    #[test]
    fn content_hash_serde_is_transparent_over_digest() {
        let digest = Sha256::of(b"payload");
        let hash = ContentHash::<BytesBag>::from_digest_unchecked(digest);
        let json = serde_json::to_string(&hash).unwrap();
        let direct = serde_json::to_string(&digest).unwrap();
        assert_eq!(json, direct);
        let back: ContentHash<BytesBag> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, hash);
    }

    #[test]
    fn content_hash_equality_is_digest_equality() {
        let a = ContentHash::<BytesBag>::of_bytes_unchecked(b"hello");
        let b = ContentHash::<BytesBag>::of_bytes_unchecked(b"hello");
        assert_eq!(a, b);
        let c = ContentHash::<BytesBag>::of_bytes_unchecked(b"world");
        assert_ne!(a, c);
    }

    #[test]
    fn content_hash_is_copy_and_clone() {
        let a = ContentHash::<BytesBag>::of_bytes_unchecked(b"hello");
        let copied = a;
        let cloned = Clone::clone(&a);
        assert_eq!(a, copied);
        assert_eq!(a, cloned);
    }

    #[test]
    fn content_hash_hash_matches_digest_hash() {
        use std::collections::HashSet;
        let a = ContentHash::<BytesBag>::of_bytes_unchecked(b"hello");
        let b = ContentHash::<BytesBag>::of_bytes_unchecked(b"hello");
        let mut set = HashSet::new();
        set.insert(a);
        assert!(set.contains(&b));
    }

    #[test]
    fn content_decode_error_utf8_displays_usefully() {
        // Built via Vec<u8> so the input isn't a compile-time literal —
        // clippy::invalid_from_utf8 rejects literals that are
        // statically-detectable invalid UTF-8.
        let bad_bytes: Vec<u8> = vec![0xff, 0xfe];
        let utf8_err = std::str::from_utf8(&bad_bytes).unwrap_err();
        let err: ContentDecodeError = utf8_err.into();
        assert!(format!("{err}").contains("not valid UTF-8"));
    }

    #[test]
    fn content_decode_error_json_displays_usefully() {
        let json_err = serde_json::from_slice::<serde_json::Value>(b"not json").unwrap_err();
        let err: ContentDecodeError = json_err.into();
        assert!(format!("{err}").contains("not valid JSON"));
    }

    #[test]
    fn content_decode_error_custom_displays_message() {
        let err = ContentDecodeError::Custom("something broke".to_string());
        assert_eq!(format!("{err}"), "content decode failed: something broke");
    }
}
