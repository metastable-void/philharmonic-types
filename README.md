# philharmonic-types

Cornerstone types for the Philharmonic workflow orchestration
system: content-addressed JSON (JCS), phantom-typed UUID identities,
SHA-256 digests, and millis-since-epoch timestamps. The
`philharmonic-*` crate family depends on this crate as its shared
vocabulary; every entity kind, every content reference, and every
identifier in the system eventually bottoms out in one of these
types.

Part of the Philharmonic workspace:
https://github.com/metastable-void/philharmonic-workspace

## What's in this crate

- **`CanonicalJson`** — JCS-canonical JSON bytes (RFC 8785). The
  invariant is that the inner bytes are already canonical (keys
  sorted at every level, numbers formatted per ECMA-262, no
  insignificant whitespace), so a `CanonicalJson` value can be
  hashed directly for content-addressing without re-serialization.
  Serde integration is human-readable-aware: JSON emits the decoded
  object, CBOR emits the raw bytes.
- **`Sha256`** — a `[u8; 32]` SHA-256 digest. `Sha256::of(bytes)`
  computes a digest; `Sha256::from_bytes_unchecked` wraps a known
  digest. Serde round-trips as hex in human-readable formats and as
  a CBOR byte string (`bstr`) in binary formats.
- **`UnixMillis`** — a signed 64-bit wall-clock timestamp in
  milliseconds since the Unix epoch. `UnixMillis::now()` reads the
  system clock.
- **`Id<T>`** — phantom-typed UUID-backed identifier. Parameterized
  on a kind marker so `EntityId<Tenant>` can't be confused with
  `EntityId<Principal>` at the type level. `InternalId` / `PublicId`
  tags distinguish internal-only vs. externally-exposed identities.
- **`Content<T>` + `ContentHash<H, T>`** — content-addressing
  primitives. `Content<T>` wraps a value + its canonical bytes +
  its hash; `ContentHash<H, T>` is a standalone hash reference
  parameterized on both the hash function and the referenced type.
- **`EntityKind`** trait + `Entity<K>` — the entity-substrate
  vocabulary (entity kinds, slot shapes, entity references) that
  the storage layer (`philharmonic-store`) and the workflow engine
  (`philharmonic-workflow`) build on.

## Design notes

- **No `unsafe`.** This crate holds the workspace's most
  dependency-touching types; it's deliberately conservative.
- **Serde shape is stable.** Changing a type's CBOR or JSON wire
  form is a breaking change and warrants a version bump; pinned
  test vectors in downstream crypto crates rely on this stability.
  `Sha256`'s `bstr` CBOR shape is the most recent example (0.3.5).
- Re-exports `uuid::Uuid` so downstream crates don't need to depend
  on `uuid` directly for the common case.

## Related crates

- [`philharmonic-store`](https://crates.io/crates/philharmonic-store)
  — storage traits built on these types.
- [`philharmonic-store-sqlx-mysql`](https://crates.io/crates/philharmonic-store-sqlx-mysql)
  — MySQL-family backend.
- [`philharmonic-policy`](https://crates.io/crates/philharmonic-policy)
  — tenant / principal / role / endpoint-config entities.
- [`philharmonic-workflow`](https://crates.io/crates/philharmonic-workflow)
  — orchestration engine.
- [`philharmonic-connector-common`](https://crates.io/crates/philharmonic-connector-common)
  — connector-layer vocabulary.

## License

**This crate is dual-licensed under `Apache-2.0 OR MPL-2.0`**;
either license is sufficient; choose whichever fits your project.

**Rationale**: We generally want our reusable Rust crates to be
under a license permissive enough to be friendly for the Rust
community as a whole, while maintaining GPL-2.0 compatibility via
the MPL-2.0 arm. This is FSF-safer for everyone than `MIT OR
Apache-2.0`, still being permissive. **This is the standard
licensing** for our reusable Rust crate projects. Someone's
`GPL-2.0-or-later` project should not be forced to drop the
`GPL-2.0` option because of our crates, while `Apache-2.0` is the
non-copyleft (permissive) license recommended by the FSF, which we
base our decisions on.

SPDX-License-Identifier: `Apache-2.0 OR MPL-2.0`

## Contributing

This crate is developed as a submodule of the Philharmonic
workspace. Workspace-wide development conventions — git workflow,
script wrappers, Rust code rules, versioning, terminology — live
in the workspace meta-repo at
[metastable-void/philharmonic-workspace](https://github.com/metastable-void/philharmonic-workspace),
authoritatively in its
[`CONTRIBUTING.md`](https://github.com/metastable-void/philharmonic-workspace/blob/main/CONTRIBUTING.md).
