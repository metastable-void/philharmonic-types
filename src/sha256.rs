use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Sha256(#[serde(with = "hex_bytes")] [u8; 32]);

impl Sha256 {
    pub const fn from_bytes_unchecked(hash: [u8; 32]) -> Self {
        Self(hash)
    }

    pub fn of(bytes: &[u8]) -> Self {
        use sha2::{Digest, Sha256 as Hasher};
        let mut h = Hasher::new();
        h.update(bytes);
        Self(h.finalize().into())
    }

    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for Sha256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "sha256:{}", hex::encode(&self.0[..8]))
    }
}

impl fmt::Display for Sha256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

pub(crate) mod hex_bytes {
    use serde::{Deserializer, Serializer, de::Visitor};

    pub struct HexVisitor<const L: usize>;

    impl<'de, const L: usize> Visitor<'de> for HexVisitor<L> {
        type Value = [u8; L];
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "Hex string expected")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let bytes = hex::decode(v).map_err(|e| E::custom(e))?;
            bytes.try_into().map_err(|_e| E::custom("Length mismatch"))
        }
    }

    pub(crate) fn serialize<S: Serializer, const L: usize>(
        bytes: &[u8; L],
        s: S,
    ) -> Result<S::Ok, S::Error> {
        s.serialize_str(&hex::encode(bytes))
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>, const L: usize>(
        d: D,
    ) -> Result<[u8; L], D::Error> {
        d.deserialize_str(HexVisitor::<L>)
    }
}

impl AsRef<[u8]> for Sha256 {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl AsRef<[u8; 32]> for Sha256 {
    fn as_ref(&self) -> &[u8; 32] {
        &self.0
    }
}

impl crate::HashFunction for Sha256 {
    type Output = Self;
    fn digest(bytes: &[u8]) -> Self::Output {
        Self::of(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // NIST FIPS 180-4 known-answer vectors.
    const EMPTY_HEX: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    const ABC_HEX: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    #[test]
    fn of_empty_matches_nist_vector() {
        assert_eq!(Sha256::of(b"").to_string(), EMPTY_HEX);
    }

    #[test]
    fn of_abc_matches_nist_vector() {
        assert_eq!(Sha256::of(b"abc").to_string(), ABC_HEX);
    }

    #[test]
    fn from_bytes_unchecked_round_trips_through_as_bytes() {
        let bytes = [0x42_u8; 32];
        let hash = Sha256::from_bytes_unchecked(bytes);
        assert_eq!(hash.as_bytes(), &bytes);
    }

    #[test]
    fn debug_shows_prefix_and_first_eight_bytes() {
        let hash = Sha256::of(b"abc");
        assert_eq!(format!("{hash:?}"), "sha256:ba7816bf8f01cfea");
    }

    #[test]
    fn display_shows_full_hex() {
        assert_eq!(format!("{}", Sha256::of(b"abc")), ABC_HEX);
    }

    #[test]
    fn serde_round_trip() {
        let hash = Sha256::of(b"abc");
        let json = serde_json::to_string(&hash).unwrap();
        assert_eq!(json, format!("\"{ABC_HEX}\""));
        let back: Sha256 = serde_json::from_str(&json).unwrap();
        assert_eq!(back, hash);
    }

    #[test]
    fn deserialize_accepts_uppercase_hex() {
        let json = format!("\"{}\"", ABC_HEX.to_uppercase());
        let hash: Sha256 = serde_json::from_str(&json).unwrap();
        assert_eq!(hash, Sha256::of(b"abc"));
    }

    #[test]
    fn deserialize_rejects_wrong_decoded_length() {
        let json = "\"deadbeef\""; // 4 bytes, not 32
        let result: Result<Sha256, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_rejects_odd_length_hex() {
        let json = "\"abc\""; // 3 hex chars — odd
        let result: Result<Sha256, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_rejects_non_hex_characters() {
        // 64 chars, but full of non-hex.
        let json = format!("\"{}\"", "z".repeat(64));
        let result: Result<Sha256, _> = serde_json::from_str(&json);
        assert!(result.is_err());
    }

    #[test]
    fn as_ref_slice_matches_as_bytes() {
        let hash = Sha256::of(b"abc");
        let slice: &[u8] = hash.as_ref();
        assert_eq!(slice, hash.as_bytes());
    }

    #[test]
    fn as_ref_array_matches_as_bytes() {
        let hash = Sha256::of(b"abc");
        let array: &[u8; 32] = hash.as_ref();
        assert_eq!(array, hash.as_bytes());
    }

    #[test]
    fn hash_function_trait_matches_inherent_of() {
        use crate::HashFunction;
        assert_eq!(Sha256::digest(b"abc"), Sha256::of(b"abc"));
    }

    #[test]
    fn equal_digests_are_eq_and_hash_the_same() {
        use std::collections::HashSet;
        let a = Sha256::of(b"abc");
        let b = Sha256::of(b"abc");
        let mut set = HashSet::new();
        set.insert(a);
        assert!(set.contains(&b));
    }
}
