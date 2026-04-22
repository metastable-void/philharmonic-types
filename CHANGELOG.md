# Changelog

All notable changes to this crate are documented in this file.

The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
this crate adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.5]

`Sha256`'s serde impl is now human-readable-aware: hex text
string for JSON / YAML / TOML (unchanged), 32-byte byte string
(CBOR major type 2) for binary formats. Mirrors the `uuid`
crate's pattern; the same `Sha256` value round-trips cleanly
through either serializer family. JSON consumers see no
behavior change. CBOR consumers previously saw a 64-char hex
tstr and will now see a 32-byte bstr; no published downstream
crate exercised the CBOR path yet, so this is observed as a
wire-format fix at the point it first matters (Phase 5 Wave A
COSE_Sign1 tokens).

New test coverage: CBOR round-trip (verifies byte-string shape
byte-by-byte), CBOR length-mismatch rejection, and a value-level
JSON-vs-CBOR interoperability test.

## [0.3.4]

Testing pass — 93 colocated unit tests added across every
source module (SHA-256 known-answer vectors, ID version
validation and cross-marker equality, UnixMillis serde and
ordering, JCS canonicalization key-sort + nested-recursion,
ContentHash / ContentValue trait round-trips,
EntityId / Identity version-validation and serde, ScalarValue
tagged-form serde). No public API changes.

Non-API notes:

- The `UnixMillis::now()` tests that call `SystemTime::now()`
  are gated with `#[cfg(not(miri))]` so `./scripts/miri-test.sh
  philharmonic-types` runs clean by default (91 tests, 0 UB).
  Full coverage under miri still possible with
  `MIRIFLAGS=-Zmiri-disable-isolation`.

## [0.3.3]

Current published baseline. Git history is the authoritative
record for this and earlier releases; future releases will be
documented going forward in this file.
