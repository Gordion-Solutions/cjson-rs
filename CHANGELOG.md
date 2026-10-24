# Changelog

All notable changes to this project are recorded here.

The format is loosely based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html) (with the usual pre-1.0 caveat that breaking changes can land in any minor version while we're in `0.x`).

## [Unreleased]

### Added

- `cJSON_Utils`: JSON Pointer (RFC 6901), JSON Patch (RFC 6902) and JSON Merge Patch (RFC 7386). All 14 `cJSONUtils_*` functions are implemented as consumers of the public cJSON API, from the RFCs, and pass cJSON's own utils test files (`misc_utils_tests`, `old_utils_tests`, `json_patch_tests` — including the upstream `json-patch-tests` corpus — and `compare_tests`). The header `cJSON_Utils.h` ships in the install and the symbols are exported from the drop-in library.
- **Not affected by CVE-2026-87933.** The use-after-free in upstream `cJSONUtils_MergePatch` (free target, then duplicate an aliased patch) cannot occur here: cjson-safe duplicates the patch before freeing the target. Verified both ways — the CVE reproduces against upstream cJSON 1.7.19 under AddressSanitizer, and the same input is clean under Miri (`cjson-rs-ffi/tests/utils_smoke.rs::merge_patch_patch_aliases_target_is_safe`).
- Drop-in packaging. The shared library carries cJSON's own names (`libcjson.so.1`, `libcjson.1.dylib`) and exports the `cJSON_*` and `cJSONUtils_*` symbols (`cjson-rs-ffi/build.rs`, `cjson.map`, `exported_symbols.txt`).
- `cjson-rs-ffi/tools/install.sh <prefix>` installs the library, upstream's `cJSON.h` and `cJSON_Utils.h` (unchanged, v1.7.19) under `include/cjson/`, `libcjson.pc` / `libcjson_utils.pc` for pkg-config and `cJSON::cjson` / `cJSON::cjson_utils` CMake targets, in cJSON's own layout. `cJSON_Utils` ships as `libcjson_utils.*` links to `libcjson` (the symbols live in the one library), so `-lcjson_utils`, `pkg-config libcjson_utils` and `find_package(cJSON)` + `cJSON::cjson_utils` all resolve with no second object loaded at runtime. `tools/stage.sh <dir>` copies the build under the real file names without installing.
- `cjson-rs-ffi/tests/c/run-install-test.sh` builds a C program against a scratch install through pkg-config and CMake — for both `libcjson` and `libcjson_utils` — and checks it loaded the installed library. Runs in CI on Linux and macOS.
- Release artefacts. `.github/workflows/release.yml` builds on Linux x86_64, Linux arm64 and macOS arm64 when a `v*` tag is pushed, tests each archive as a user would (unpack, `install.sh`, build C through pkg-config), and attaches `cjson-rs-<version>-<platform>.tar.gz`, its `.sha256` and a combined `SHA256SUMS` to the GitHub release. "Run workflow" builds and tests without publishing. Archives are made by `cjson-rs-ffi/tools/package.sh`; each carries an `install.sh` that needs no Rust toolchain.

### Changed

- Prepared for crates.io. The C-ABI crate publishes as **`cjson-safe`** (builds the drop-in `libcjson`); the safe engine publishes as **`cjson-safe-core`** (unpromoted dependency). The repository stays `cjson-rs` and the import names are unchanged (`cjson` for the drop-in, `cjson_rs` for the core). The path dependency is now versioned so both are publishable (publish `cjson-safe-core` first, then `cjson-safe`). `authors` is now "Gordion Solutions"; licence stays MIT. Workspace version is `0.3.0`; `cJSON_Version()` returns `cjson-safe 0.3.0`. The `.pc`/CMake ABI version stays 1.7.19.
- The README leads with the drop-in story and one honest line sending Rust users to `serde_json`.
- The conformance and C smoke runners link the staged library, so they exercise the same file names a user gets.
- MSRV lowered to 1.74 (was declared 1.94). Verified: the whole workspace builds and all tests pass on Rust 1.74.0. CI gained an MSRV job (build + test on 1.74), a `macos-latest` leg of the Rust test suite, and a short libFuzzer+ASan run (60s per target) of `fuzz_parse` and `fuzz_ffi_roundtrip`.

### Fixed

- `cJSON_ReplaceItemInObject` / `cJSON_ReplaceItemInObjectCaseSensitive` now set the replacement's key to the passed name, matching cJSON. Previously the replacement kept whatever key it already carried (e.g. a node from `cJSON_Duplicate`), so replacing a member with such a node could change the member's name. Surfaced by RFC 6902 `add` replacing an existing member.

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
