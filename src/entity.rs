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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    // Two disjoint test entity kinds for slot-of-T and Debug-NAME checks.
    struct TestKind;
    impl Entity for TestKind {
        const KIND: Uuid = uuid::uuid!("00000000-0000-0000-0000-000000000001");
        const NAME: &'static str = "test_kind";
        const CONTENT_SLOTS: &'static [ContentSlot] = &[];
        const ENTITY_SLOTS: &'static [EntitySlot] = &[];
        const SCALAR_SLOTS: &'static [ScalarSlot] = &[];
    }

    struct OtherKind;
    impl Entity for OtherKind {
        const KIND: Uuid = uuid::uuid!("00000000-0000-0000-0000-000000000002");
        const NAME: &'static str = "other_kind";
        const CONTENT_SLOTS: &'static [ContentSlot] = &[];
        const ENTITY_SLOTS: &'static [EntitySlot] = &[];
        const SCALAR_SLOTS: &'static [ScalarSlot] = &[];
    }

    // ------- Slot value-type tests -------

    #[test]
    fn content_slot_new_captures_name() {
        let slot = ContentSlot::new("display_name");
        assert_eq!(slot.name, "display_name");
    }

    #[test]
    fn entity_slot_of_captures_target_kind_and_pinning() {
        let slot = EntitySlot::of::<TestKind>("tenant", SlotPinning::Pinned);
        assert_eq!(slot.name, "tenant");
        assert_eq!(slot.target_kind, TestKind::KIND);
        assert_eq!(slot.pinning, SlotPinning::Pinned);
    }

    #[test]
    fn entity_slot_of_different_kinds_captures_different_target_kinds() {
        let test = EntitySlot::of::<TestKind>("x", SlotPinning::Latest);
        let other = EntitySlot::of::<OtherKind>("x", SlotPinning::Latest);
        assert_ne!(test.target_kind, other.target_kind);
    }

    #[test]
    fn scalar_slot_new_captures_all_fields() {
        let slot = ScalarSlot::new("flag", ScalarType::Bool, true);
        assert_eq!(slot.name, "flag");
        assert_eq!(slot.ty, ScalarType::Bool);
        assert!(slot.indexed);

        let slot = ScalarSlot::new("count", ScalarType::I64, false);
        assert_eq!(slot.ty, ScalarType::I64);
        assert!(!slot.indexed);
    }

    #[test]
    fn slot_pinning_equality() {
        assert_eq!(SlotPinning::Pinned, SlotPinning::Pinned);
        assert_ne!(SlotPinning::Pinned, SlotPinning::Latest);
    }

    // ------- ScalarValue tests -------

    #[test]
    fn scalar_value_ty_matches_variant() {
        assert_eq!(ScalarValue::Bool(true).ty(), ScalarType::Bool);
        assert_eq!(ScalarValue::I64(42).ty(), ScalarType::I64);
    }

    #[test]
    fn scalar_value_bool_serde_round_trip() {
        let v = ScalarValue::Bool(true);
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, r#"{"type":"bool","value":true}"#);
        let back: ScalarValue = serde_json::from_str(&json).unwrap();
        assert_eq!(back, v);
    }

    #[test]
    fn scalar_value_i64_serde_round_trip() {
        let v = ScalarValue::I64(-7);
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, r#"{"type":"i64","value":-7}"#);
        let back: ScalarValue = serde_json::from_str(&json).unwrap();
        assert_eq!(back, v);
    }

    #[test]
    fn scalar_value_rejects_unknown_tag() {
        let json = r#"{"type":"string","value":"nope"}"#;
        let result: Result<ScalarValue, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    // ------- Identity / EntityId tests -------

    fn make_identity() -> Identity {
        Identity {
            internal: Uuid::now_v7(),
            public: Uuid::new_v4(),
        }
    }

    #[test]
    fn identity_typed_validates_both_versions() {
        let identity = make_identity();
        let typed: EntityId<TestKind> = identity.typed().unwrap();
        assert_eq!(typed.untyped(), identity);
    }

    #[test]
    fn identity_typed_rejects_wrong_internal_version() {
        let bad = Identity {
            internal: Uuid::new_v4(), // v4, not v7
            public: Uuid::new_v4(),
        };
        let result: Result<EntityId<TestKind>, _> = bad.typed();
        assert!(matches!(result, Err(IdentityKindError::Internal(_))));
    }

    #[test]
    fn identity_typed_rejects_wrong_public_version() {
        let bad = Identity {
            internal: Uuid::now_v7(),
            public: Uuid::now_v7(), // v7, not v4
        };
        let result: Result<EntityId<TestKind>, _> = bad.typed();
        assert!(matches!(result, Err(IdentityKindError::Public(_))));
    }

    #[test]
    fn entity_id_internal_and_public_accessors() {
        let identity = make_identity();
        let typed: EntityId<TestKind> = identity.typed().unwrap();
        assert_eq!(typed.internal().as_uuid(), identity.internal);
        assert_eq!(typed.public().as_uuid(), identity.public);
    }

    #[test]
    fn entity_id_debug_includes_kind_name() {
        let typed: EntityId<TestKind> = make_identity().typed().unwrap();
        let debug = format!("{typed:?}");
        assert!(debug.starts_with("EntityId<test_kind>("));
    }

    #[test]
    fn entity_id_eq_compares_internal_only() {
        let internal = Uuid::now_v7();
        let a = Identity {
            internal,
            public: Uuid::new_v4(),
        }
        .typed::<TestKind>()
        .unwrap();
        let b = Identity {
            internal,
            public: Uuid::new_v4(),
        }
        .typed::<TestKind>()
        .unwrap();
        // Documented behavior: PartialEq compares internal IDs only.
        // Two EntityId<T> values with identical internal but different
        // public compare equal.
        assert_eq!(a, b);
    }

    #[test]
    fn entity_id_hash_uses_internal() {
        let internal = Uuid::now_v7();
        let a = Identity {
            internal,
            public: Uuid::new_v4(),
        }
        .typed::<TestKind>()
        .unwrap();
        let b = Identity {
            internal,
            public: Uuid::new_v4(),
        }
        .typed::<TestKind>()
        .unwrap();
        let mut set = HashSet::new();
        set.insert(a);
        assert!(set.contains(&b));
    }

    #[test]
    fn entity_id_serde_round_trip() {
        let typed: EntityId<TestKind> = make_identity().typed().unwrap();
        let json = serde_json::to_string(&typed).unwrap();
        // Serializes as Identity — object with `internal` and `public` keys.
        assert!(json.contains("\"internal\":"));
        assert!(json.contains("\"public\":"));
        let back: EntityId<TestKind> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, typed);
    }

    #[test]
    fn entity_id_deserialize_rejects_wrong_internal_version() {
        let bad = Identity {
            internal: Uuid::new_v4(), // wrong version
            public: Uuid::new_v4(),
        };
        let json = serde_json::to_string(&bad).unwrap();
        let result: Result<EntityId<TestKind>, _> = serde_json::from_str(&json);
        assert!(result.is_err());
    }

    #[test]
    fn entity_id_copy_and_clone_work() {
        let typed: EntityId<TestKind> = make_identity().typed().unwrap();
        let copied = typed;
        let cloned = Clone::clone(&typed);
        assert_eq!(typed, copied);
        assert_eq!(typed, cloned);
    }

    #[test]
    fn identity_kind_error_variants_display() {
        let internal_err = IdentityKindError::Internal(crate::IdKindError {
            expected: 7,
            actual: 4,
        });
        let public_err = IdentityKindError::Public(crate::IdKindError {
            expected: 4,
            actual: 7,
        });
        assert!(format!("{internal_err}").contains("internal"));
        assert!(format!("{public_err}").contains("public"));
    }
}
