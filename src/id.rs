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

#[derive(Debug, thiserror::Error)]
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
