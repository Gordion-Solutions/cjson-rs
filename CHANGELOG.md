# Changelog

All notable changes to this project are recorded here.

The format is loosely based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html) (with the usual pre-1.0 caveat that breaking changes can land in any minor version while we're in `0.x`).

## [Unreleased]

### Added

- `cJSON_Utils`: JSON Pointer (RFC 6901), JSON Patch (RFC 6902) and JSON Merge Patch (RFC 7386). All 14 `cJSONUtils_*` functions are implemented as consumers of the public cJSON API, from the RFCs, and pass cJSON's own utils test files (`misc_utils_tests`, `old_utils_tests`, `json_patch_tests` — including the upstream `json-patch-tests` corpus — and `compare_tests`). The header `cJSON_Utils.h` ships in the install and the symbols are exported from the drop-in library.
- `install.sh` now ships `cJSON_Utils`: it installs `cJSON_Utils.h`, writes `lib/pkgconfig/libcjson_utils.pc`, adds a `cJSON::cjson_utils` CMake target, and creates `libcjson_utils.so.1` / `libcjson_utils.1.dylib` as links to `libcjson`. Consumers that link `-lcjson_utils`, use `pkg-config libcjson_utils`, or `find_package(cJSON)` + `cJSON::cjson_utils` all resolve, with no second object loaded at runtime. The install-test (`run-install-test.sh`, run in CI on Linux and macOS) now links the probe through `libcjson_utils` by pkg-config and CMake as well.
- **Not affected by CVE-2026-87933.** The use-after-free in upstream `cJSONUtils_MergePatch` (free target, then duplicate an aliased patch) cannot occur here: cjson-rs duplicates the patch before freeing the target. Verified both ways — the CVE reproduces against upstream cJSON 1.7.19 under AddressSanitizer, and the same input is clean in cjson-rs under Miri (`cjson-rs-ffi/tests/utils_smoke.rs::merge_patch_patch_aliases_target_is_safe`).
- Drop-in packaging. The shared library now carries cJSON's own names (`libcjson.so.1`, `libcjson.1.dylib`) and exports the `cJSON_*` and `cJSONUtils_*` symbols (`cjson-rs-ffi/build.rs`, `cjson.map`, `exported_symbols.txt`).

### Fixed

- `cJSON_ReplaceItemInObject` / `cJSON_ReplaceItemInObjectCaseSensitive` now set the replacement's key to the passed name, matching cJSON. Previously the replacement kept whatever key it already carried (e.g. a node from `cJSON_Duplicate`), so replacing a member with such a node could change the member's name. Surfaced by RFC 6902 `add` replacing an existing member.
- `cjson-rs-ffi/tools/install.sh <prefix>` installs the library, upstream's `cJSON.h` (unchanged, v1.7.19) as `include/cjson/cJSON.h`, `libcjson.pc` for pkg-config and `cJSON::cjson` CMake package files, in cJSON's own layout. `tools/stage.sh <dir>` copies the build under the real file names without installing.
- `cjson-rs-ffi/tests/c/run-install-test.sh` builds a C program against a scratch install through pkg-config and CMake and checks it loaded the installed library. Runs in CI on Linux and macOS.

### Changed

- The conformance and C smoke runners link the staged library, so they exercise the same file names a user gets.
- MSRV lowered from 1.74 (was declared 1.94). Verified: the whole workspace builds and all tests pass on Rust 1.74.0. CI gained an MSRV job that builds and tests on 1.74, a `macos-latest` leg of the Rust test suite, and a short libFuzzer+ASan run (60s per target) of `fuzz_parse` and `fuzz_ffi_roundtrip`.

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
