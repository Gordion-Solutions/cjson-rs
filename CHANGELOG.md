# Changelog

All notable changes to this project are recorded here.

The format is loosely based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html) (with the usual pre-1.0 caveat that breaking changes can land in any minor version while we're in `0.x`).

## [Unreleased]

### Added

- Drop-in packaging. The shared library now carries cJSON's own names (`libcjson.so.1`, `libcjson.1.dylib`) and exports only the 78 `cJSON_*` symbols (`cjson-rs-ffi/build.rs`, `cjson.map`, `exported_symbols.txt`).
- `cjson-rs-ffi/tools/install.sh <prefix>` installs the library, upstream's `cJSON.h` (unchanged, v1.7.19) as `include/cjson/cJSON.h`, `libcjson.pc` for pkg-config and `cJSON::cjson` CMake package files, in cJSON's own layout. `tools/stage.sh <dir>` copies the build under the real file names without installing.
- `cjson-rs-ffi/tests/c/run-install-test.sh` builds a C program against a scratch install through pkg-config and CMake and checks it loaded the installed library. Runs in CI on Linux and macOS.

- Release artefacts. `.github/workflows/release.yml` builds on Linux x86_64, Linux arm64 and macOS arm64 when a `v*` tag is pushed, tests each archive as a user would (unpack, `install.sh`, build C through pkg-config), and attaches `cjson-rs-<version>-<platform>.tar.gz`, its `.sha256` and a combined `SHA256SUMS` to the GitHub release. "Run workflow" builds and tests without publishing. Archives are made by `cjson-rs-ffi/tools/package.sh`; each carries an `install.sh` that needs no Rust toolchain.

### Changed

- The conformance and C smoke runners link the staged library, so they exercise the same file names a user gets.

## [0.2.0] — 2026-05-19

### Added

- `cJSON_PrintPreallocated` is now a real implementation. Prints the value into the caller-supplied buffer; returns `true` on success, `false` on NULL inputs, non-positive length, or undersized buffer. Brings the FFI surface from 77/78 to 78/78 cJSON.h functions implemented.
- `cjson_rs::parse_prefix` — new public Rust API. Parses a single JSON value from the prefix of a buffer that may contain trailing whitespace, garbage, or further values. Returns the value plus the byte offset immediately after its last consumed byte.

### Changed

- `cJSON_ParseWithOpts` and `cJSON_ParseWithLengthOpts` now honour the `require_null_terminated` flag. When `false`, the parser accepts valid-prefix-then-trailing-bytes input (the documented headline use case of `WithOpts`) and reports `parse_end` correctly. Previously the flag was ignored and trailing bytes were always rejected.
- Upstream cJSON conformance: **64/65 → 65/65 (100%)** on the public-API subset.

### Documentation

- README and METHODOLOGY now reflect 100% conformance and 78/78 functions.
- Fuzz claim reframed as a baseline (~4.5M iterations is modest; extended multi-day runs are planned with funded work).
- `cjson-rs-ffi/src/panic_guard.rs` documents the invariant that the safe core must remain stateless per-parse for the `AssertUnwindSafe` wrap to remain sound.

## [0.0.1] — Initial release

Initial public release of the clean-room Rust implementation with C ABI shim.
