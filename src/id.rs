use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

use std::{
    fmt::{Debug, Display},
    hash::Hash,
    marker::PhantomData,
};

const KIND_INTERNAL: u8 = 0;
const KIND_PUBLIC: u8 = 1;

#[derive(Serialize)]
#[repr(transparent)]
#[serde(transparent)]
/// `KIND` is one of
///
/// * `KIND_INTERNAL` (UUIDv7, for time-ordered internal addressing), or
/// * `KIND_PUBLIC` (UUIDv4, for opaque external references);
///
/// use the `InternalId<T>` and `PublicId<T>` aliases
/// rather than naming `Id<T, KIND>` directly.
pub struct Id<T: ?Sized, const KIND: u8> {
    uuid: Uuid,

    #[serde(skip)]
    // fn() -> T makes this Send+Sync regardless of T
    _phantom: PhantomData<fn() -> T>,
}

impl<T: ?Sized, const KIND: u8> Id<T, KIND> {
    pub const fn as_uuid(&self) -> Uuid {
        self.uuid
    }

    pub const fn as_bytes(&self) -> &[u8; 16] {
        self.uuid.as_bytes()
    }

    /// Construct an Id from a UUID without validating its version against KIND.
    /// Only use when you have an external invariant guaranteeing the version,
    /// such as reading from a database column whose schema enforces it.
    pub fn from_uuid_unchecked(uuid: Uuid) -> Self {
        Self {
            uuid,
            _phantom: PhantomData,
        }
    }
}

impl<T: ?Sized, const KIND: u8> Debug for Id<T, KIND> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match KIND {
            KIND_INTERNAL => write!(f, "InternalId({})", self.uuid),
            KIND_PUBLIC => write!(f, "PublicId({})", self.uuid),
            _k => write!(f, "Id({})", self.uuid),
        }
    }
}

impl<T: ?Sized, const KIND: u8> Display for Id<T, KIND> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.uuid)
    }
}

impl<T: ?Sized, const KIND: u8> Clone for Id<T, KIND> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized, const KIND: u8> Copy for Id<T, KIND> {}

impl<T1: ?Sized, T2: ?Sized, const KIND1: u8, const KIND2: u8> PartialEq<Id<T2, KIND2>>
    for Id<T1, KIND1>
{
    fn eq(&self, other: &Id<T2, KIND2>) -> bool {
        self.uuid == other.uuid
    }
}

impl<T: ?Sized, const KIND: u8> Eq for Id<T, KIND> {}

impl<T: ?Sized, const KIND: u8> Hash for Id<T, KIND> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.uuid.hash(state)
    }
}

pub type InternalId<T> = Id<T, { KIND_INTERNAL }>;
pub type PublicId<T> = Id<T, { KIND_PUBLIC }>;

impl<T: ?Sized> InternalId<T> {
    pub const KIND: u8 = KIND_INTERNAL;

    pub fn new_v7() -> Self {
        Self {
            uuid: Uuid::now_v7(),
            _phantom: PhantomData,
        }
    }

    pub fn from_uuid(uuid: Uuid) -> Result<Self, IdKindError> {
        if uuid.get_version_num() != 7 {
            return Err(IdKindError {
                expected: 7,
                actual: uuid.get_version_num(),
            });
        }
        Ok(Self {
            uuid,
            _phantom: PhantomData,
        })
    }

    pub fn from_bytes(bytes: [u8; 16]) -> Result<Self, IdKindError> {
        Self::from_uuid(Uuid::from_bytes(bytes))
    }
}

impl<T: ?Sized> PublicId<T> {
    pub const KIND: u8 = KIND_PUBLIC;

    pub fn new_v4() -> Self {
        Self {
            uuid: Uuid::new_v4(),
            _phantom: PhantomData,
        }
    }

    pub fn from_uuid(uuid: Uuid) -> Result<Self, IdKindError> {
        if uuid.get_version_num() != 4 {
            return Err(IdKindError {
                expected: 4,
                actual: uuid.get_version_num(),
            });
        }
        Ok(Self {
            uuid,
            _phantom: PhantomData,
        })
    }

    pub fn from_bytes(bytes: [u8; 16]) -> Result<Self, IdKindError> {
        Self::from_uuid(Uuid::from_bytes(bytes))
    }
}

#[derive(Debug, thiserror::Error, Clone)]
#[error("UUID version mismatch: expected v{expected}, got v{actual}")]
pub struct IdKindError {
    pub expected: usize,
    pub actual: usize,
}

impl<'de, T: ?Sized> Deserialize<'de> for InternalId<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let uuid = Uuid::deserialize(d)?;
        Self::from_uuid(uuid).map_err(serde::de::Error::custom)
    }
}

impl<'de, T: ?Sized> Deserialize<'de> for PublicId<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let uuid = Uuid::deserialize(d)?;
        Self::from_uuid(uuid).map_err(serde::de::Error::custom)
    }
}

impl<T: ?Sized, const KIND: u8> AsRef<[u8]> for Id<T, KIND> {
    fn as_ref(&self) -> &[u8] {
        self.uuid.as_bytes()
    }
}

impl<T: ?Sized, const KIND: u8> AsRef<[u8; 16]> for Id<T, KIND> {
    fn as_ref(&self) -> &[u8; 16] {
        self.uuid.as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    // Two disjoint markers to probe cross-type behavior.
    struct MarkerA;
    struct MarkerB;

    #[test]
    fn internal_id_new_v7_produces_v7() {
        let id = InternalId::<MarkerA>::new_v7();
        assert_eq!(id.as_uuid().get_version_num(), 7);
    }

    #[test]
    fn public_id_new_v4_produces_v4() {
        let id = PublicId::<MarkerA>::new_v4();
        assert_eq!(id.as_uuid().get_version_num(), 4);
    }

    #[test]
    fn internal_id_from_uuid_rejects_v4() {
        let v4 = Uuid::new_v4();
        let result = InternalId::<MarkerA>::from_uuid(v4);
        let err = result.unwrap_err();
        assert_eq!(err.expected, 7);
        assert_eq!(err.actual, 4);
    }

    #[test]
    fn public_id_from_uuid_rejects_v7() {
        let v7 = Uuid::now_v7();
        let result = PublicId::<MarkerA>::from_uuid(v7);
        let err = result.unwrap_err();
        assert_eq!(err.expected, 4);
        assert_eq!(err.actual, 7);
    }

    #[test]
    fn internal_id_from_uuid_rejects_nil_uuid() {
        // Nil UUID has version 0 — rejected by from_uuid.
        let result = InternalId::<MarkerA>::from_uuid(Uuid::nil());
        assert!(result.is_err());
    }

    #[test]
    fn from_bytes_forwards_to_from_uuid() {
        let v7 = Uuid::now_v7();
        let id = InternalId::<MarkerA>::from_bytes(*v7.as_bytes()).unwrap();
        assert_eq!(id.as_uuid(), v7);
    }

    #[test]
    fn from_bytes_rejects_wrong_version() {
        let v4 = Uuid::new_v4();
        let result = InternalId::<MarkerA>::from_bytes(*v4.as_bytes());
        assert!(result.is_err());
    }

    #[test]
    fn from_uuid_unchecked_skips_version_validation() {
        let v4 = Uuid::new_v4();
        // `from_uuid_unchecked` is explicitly for cases the caller has
        // already validated; it must not re-check.
        let id = InternalId::<MarkerA>::from_uuid_unchecked(v4);
        assert_eq!(id.as_uuid(), v4);
    }

    #[test]
    fn same_uuid_cross_marker_compares_equal() {
        let v7 = Uuid::now_v7();
        let a = InternalId::<MarkerA>::from_uuid(v7).unwrap();
        let b = InternalId::<MarkerB>::from_uuid(v7).unwrap();
        // Cross-kind PartialEq impl compares UUIDs ignoring markers.
        assert_eq!(a, b);
    }

    #[test]
    fn hash_matches_uuid_hash() {
        let v7 = Uuid::now_v7();
        let a = InternalId::<MarkerA>::from_uuid(v7).unwrap();
        let mut set = HashSet::new();
        set.insert(a);
        let b = InternalId::<MarkerA>::from_uuid(v7).unwrap();
        assert!(set.contains(&b));
    }

    #[test]
    fn debug_format_labels_internal_kind() {
        let id = InternalId::<MarkerA>::new_v7();
        let debug = format!("{id:?}");
        assert!(debug.starts_with("InternalId("));
        assert!(debug.ends_with(')'));
    }

    #[test]
    fn debug_format_labels_public_kind() {
        let id = PublicId::<MarkerA>::new_v4();
        let debug = format!("{id:?}");
        assert!(debug.starts_with("PublicId("));
        assert!(debug.ends_with(')'));
    }

    #[test]
    fn display_is_bare_uuid_string() {
        let id = InternalId::<MarkerA>::new_v7();
        assert_eq!(format!("{id}"), id.as_uuid().to_string());
    }

    #[test]
    fn internal_id_serialize_as_bare_uuid() {
        let id = InternalId::<MarkerA>::new_v7();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, format!("\"{}\"", id.as_uuid()));
    }

    #[test]
    fn internal_id_deserialize_validates_version() {
        let v4 = Uuid::new_v4();
        let json = format!("\"{v4}\"");
        let result: Result<InternalId<MarkerA>, _> = serde_json::from_str(&json);
        assert!(result.is_err());
    }

    #[test]
    fn internal_id_deserialize_accepts_v7() {
        let v7 = Uuid::now_v7();
        let json = format!("\"{v7}\"");
        let id: InternalId<MarkerA> = serde_json::from_str(&json).unwrap();
        assert_eq!(id.as_uuid(), v7);
    }

    #[test]
    fn public_id_deserialize_validates_version() {
        let v7 = Uuid::now_v7();
        let json = format!("\"{v7}\"");
        let result: Result<PublicId<MarkerA>, _> = serde_json::from_str(&json);
        assert!(result.is_err());
    }

    #[test]
    fn as_ref_slice_returns_uuid_bytes() {
        let id = InternalId::<MarkerA>::new_v7();
        let slice: &[u8] = id.as_ref();
        assert_eq!(slice, id.as_uuid().as_bytes());
    }

    #[test]
    fn as_ref_array_returns_uuid_bytes() {
        let id = InternalId::<MarkerA>::new_v7();
        let array: &[u8; 16] = id.as_ref();
        assert_eq!(array, id.as_uuid().as_bytes());
    }

    #[test]
    fn id_kind_error_display_mentions_versions() {
        let err = IdKindError {
            expected: 7,
            actual: 4,
        };
        let msg = format!("{err}");
        assert!(msg.contains("v7"));
        assert!(msg.contains("v4"));
    }

    #[test]
    fn copy_and_clone_work() {
        let id = InternalId::<MarkerA>::new_v7();
        let copied = id;
        // Use FQCS to exercise the Clone impl explicitly without
        // tripping clippy::clone_on_copy (which rightly flags
        // `id.clone()` on a Copy type as redundant in real code).
        let cloned = Clone::clone(&id);
        assert_eq!(id, copied);
        assert_eq!(id, cloned);
    }
}
