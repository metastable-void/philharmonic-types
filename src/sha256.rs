use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Sha256(#[serde(with = "hex_bytes")] pub [u8; 32]);

impl Sha256 {
    pub fn of(bytes: &[u8]) -> Self {
        use sha2::{Digest, Sha256 as Hasher};
        let mut h = Hasher::new();
        h.update(bytes);
        Self(h.finalize().into())
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
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
