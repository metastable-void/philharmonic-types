use crate::{InternalId, PublicId};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

use std::marker::PhantomData;

/// A kind of entity that can be stored in the entity substrate.
///
/// Each implementor declares its kind UUID (a globally-unique, stable
/// identifier minted once at type-authoring time), a human-readable name,
/// and the slots it has — content references, entity references, and
/// queryable scalar values.
///
/// Slot names are local to the entity kind; they don't need to be globally
/// unique. The kind UUID and the slot name together identify a slot
/// unambiguously.
pub trait Entity: Sized {
    /// Globally-unique identifier for this entity kind.
    /// Generated once at authoring time as a UUIDv4; never changes.
    const KIND: Uuid;

    /// Human-readable name for this kind, used in debug output and tooling.
    /// Not load-bearing for identity; kinds are identified by KIND.
    const NAME: &'static str;

    /// The content-addressed slots this entity kind has.
    const CONTENT_SLOTS: &'static [ContentSlot];

    /// The entity-reference slots this entity kind has.
    const ENTITY_SLOTS: &'static [EntitySlot];

    /// The queryable scalar slots this entity kind has.
    const SCALAR_SLOTS: &'static [ScalarSlot];
}

/// Declaration of a content-addressed slot on an entity revision.
///
/// A content slot holds a `ContentHash<T>` for some content type `T`;
/// the type isn't recorded in the slot declaration because content is
/// type-erased at the storage layer (it's just bytes). The consumer of
/// the slot is responsible for knowing what content type to decode it as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContentSlot {
    pub name: &'static str,
}

impl ContentSlot {
    pub const fn new(name: &'static str) -> Self {
        Self { name }
    }
}

/// Declaration of an entity-reference slot on an entity revision.
///
/// An entity slot holds a reference to another entity, optionally pinned
/// to a specific revision. The target_kind is the KIND UUID of the entity
/// kind this slot may point at, recorded for documentation and for
/// validation by domain code (the storage substrate doesn't enforce it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntitySlot {
    pub name: &'static str,
    pub target_kind: Uuid,
    pub pinning: SlotPinning,
}

impl EntitySlot {
    /// Declare an entity slot pointing at entities of kind `T`.
    ///
    /// The `T: Entity` bound ensures the target kind is a real entity
    /// kind in the system; renaming or removing `T` causes a compile
    /// error here, which keeps slot declarations honest.
    pub const fn of<T: Entity>(name: &'static str, pinning: SlotPinning) -> Self {
        Self {
            name,
            target_kind: T::KIND,
            pinning,
        }
    }
}

/// How an entity reference resolves: to a specific revision, or to
/// whatever is current.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotPinning {
    /// The reference includes a revision_seq; reads always return that
    /// specific revision regardless of what newer revisions exist.
    Pinned,
    /// The reference is to the entity only; reads return the latest
    /// revision at read time.
    Latest,
}

/// Declaration of a scalar slot on an entity revision.
///
/// Scalars are small typed values stored directly on the revision rather
/// than via content-addressing. They're useful for fields that need to be
/// queryable (filtered, sorted, indexed) without going through the content
/// store. The `indexed` flag indicates whether the storage substrate
/// should maintain a secondary index on this scalar for query performance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScalarSlot {
    pub name: &'static str,
    pub ty: ScalarType,
    pub indexed: bool,
}

impl ScalarSlot {
    pub const fn new(name: &'static str, ty: ScalarType, indexed: bool) -> Self {
        Self { name, ty, indexed }
    }
}

/// The type of a scalar slot's value.
///
/// Deliberately narrow. Strings are not a scalar type — text content
/// belongs in content slots (via `CanonicalJson`), enum-like values
/// belong in `I64` with the variants defined in Rust, and external
/// references belong in `EntitySlot`. If you find yourself wanting a
/// string scalar, the value probably wants a different home.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScalarType {
    Bool,
    I64,
}

/// A scalar value stored in or read from a scalar slot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ScalarValue {
    Bool(bool),
    I64(i64),
}

impl ScalarValue {
    pub fn ty(&self) -> ScalarType {
        match self {
            Self::Bool(_) => ScalarType::Bool,
            Self::I64(_) => ScalarType::I64,
        }
    }
}

/// The untyped identity pair for an entity: its internal (UUIDv7) and
/// public (UUIDv4) identifiers.
///
/// This is the storage-boundary form. Callers who know the entity kind
/// should generally hold an `EntityId<T>` instead, which carries the
/// kind in its type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Identity {
    /// UUIDv7, used for internal storage and ordering.
    pub internal: Uuid,
    /// UUIDv4, used for opaque external references.
    pub public: Uuid,
}

impl Identity {
    /// Promote this untyped identity to a typed `EntityId<T>`,
    /// validating that the UUIDs have the expected versions.
    pub fn typed<T: Entity>(self) -> Result<EntityId<T>, IdentityKindError> {
        let internal = InternalId::from_uuid(self.internal).map_err(IdentityKindError::Internal)?;
        let public = PublicId::from_uuid(self.public).map_err(IdentityKindError::Public)?;
        Ok(EntityId {
            internal,
            public,
            _phantom: PhantomData,
        })
    }
}

/// A typed identity pair for an entity of kind `T`.
///
/// Combines the internal and public IDs with a phantom type parameter
/// indicating the entity kind. Cannot be constructed without going through
/// either a fresh mint (via the storage layer) or a validated promotion
/// from `Identity`.
pub struct EntityId<T: Entity> {
    internal: InternalId<T>,
    public: PublicId<T>,
    _phantom: PhantomData<fn() -> T>,
}

impl<T: Entity> EntityId<T> {
    /// The internal (UUIDv7) ID.
    pub fn internal(&self) -> InternalId<T> {
        self.internal
    }

    /// The public (UUIDv4) ID.
    pub fn public(&self) -> PublicId<T> {
        self.public
    }

    /// Demote to the untyped identity pair.
    pub fn untyped(&self) -> Identity {
        Identity {
            internal: self.internal.as_uuid(),
            public: self.public.as_uuid(),
        }
    }
}

// Manual impls because PhantomData<fn() -> T> doesn't constrain T.
impl<T: Entity> Clone for EntityId<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: Entity> Copy for EntityId<T> {}

impl<T: Entity> PartialEq for EntityId<T> {
    fn eq(&self, other: &Self) -> bool {
        // Comparing internal IDs is sufficient; public IDs are derived
        // (in the sense that the pair is minted together and one implies
        // the other within an entity's lifetime).
        self.internal == other.internal
    }
}
impl<T: Entity> Eq for EntityId<T> {}

impl<T: Entity> std::hash::Hash for EntityId<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.internal.hash(state);
    }
}

impl<T: Entity> std::fmt::Debug for EntityId<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EntityId<{}>({})", T::NAME, self.internal.as_uuid())
    }
}

#[derive(Debug, thiserror::Error, Clone)]
pub enum IdentityKindError {
    #[error("internal ID is not UUIDv7: {0}")]
    Internal(crate::IdKindError),
    #[error("public ID is not UUIDv4: {0}")]
    Public(crate::IdKindError),
}

impl<T: Entity> Serialize for EntityId<T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.untyped().serialize(s)
    }
}

impl<'de, T: Entity> Deserialize<'de> for EntityId<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Identity::deserialize(d)?
            .typed()
            .map_err(serde::de::Error::custom)
    }
}
