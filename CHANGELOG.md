# Changelog

All notable changes to this crate are documented in this file.

The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
this crate adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
