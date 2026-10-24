# Roadmap

This is the high-level direction for cjson-safe (the `cjson-rs` repository). It
is a living document; dates are intentionally omitted. Trackable work is kept
as GitHub Issues against a release milestone; this file is the narrative.

cjson-safe is a **memory-safe drop-in for C's libcjson** — not a Rust JSON
library. Rust users should reach for [`serde_json`](https://crates.io/crates/serde_json).

## Done

- Drop-in packaging: cJSON's own library names, SONAME, symbol export, header
  layout, pkg-config and CMake package files.
- `cJSON_Utils`: JSON Pointer (RFC 6901), JSON Patch (RFC 6902) and JSON Merge
  Patch (RFC 7386), passing cJSON's own utils test suites.
- **Not affected by CVE-2026-87933** (use-after-free in `cJSONUtils_MergePatch`):
  verified vulnerable upstream under ASan, verified clean here under Miri.
- Proof against Eclipse Mosquitto 2.1.2 on Linux x86_64 (399/404, matching
  upstream; the rest are build/test-harness limitations, documented).
- Benchmarks vs cJSON 1.7.19: print 3–4.6× faster, delete at parity, parse
  improved (numbers ~1.2×, objects ~3× — see below).
- Release-artefact workflow; crates.io preparation (`cjson-safe` +
  `cjson-safe-core`).
- Hardening: MSRV 1.74, macOS in CI, short libFuzzer+ASan run in CI.

## Near term

- **Parse performance — direct-into-`cJSON` parser.** The parser currently
  builds an intermediate Rust `Value` tree before the `cJSON` tree. Parsing
  straight into `cJSON` nodes would make numbers parse *faster* than cJSON and
  bring object parsing from ~3× to ~2×. Gated on careful allocation-failure and
  error-path cleanup (free the half-built tree correctly) with ASan coverage of
  those paths — this touches the memory-safety guarantee, so it is a dedicated,
  separately-fuzzed effort, not a quick change.
- **CI proof on Linux arm64 and macOS arm64.** The release workflow can build
  and test these; they have not been exercised yet (needs the workflows to run).
- **Refresh `bench/README.md`** with full-dataset numbers from the current
  build.
- **Extended fuzzing.** Promote the CI smoke fuzz to scheduled multi-day
  libFuzzer+ASan runs and keep a corpus.

## Later

- **Sub-2× object parsing.** Avoid the second string allocation (the tokeniser
  allocates a `String`, then the C buffer is a second copy). This means a
  borrowed-string `Value`, a public Rust-API change — weigh against the fact
  that the Rust API is explicitly not the product.
- **Optional upstream-compatible number printing.** An opt-in mode that
  reproduces cJSON's `%1.15g` output for consumers that byte-compare printed
  JSON against upstream. Default stays the exact, lossless shortest round-trip.
- **Publish to crates.io** (`cjson-safe-core` first, then `cjson-safe`) once the
  near-term items settle.

## 1.0 criteria

1.0 is an ABI-stability promise. Before it:

- Mosquitto proof green on more than one platform.
- `cJSON_Utils` shipped and stable (done).
- Fuzzing with real history (not just a CI smoke run).
- Parse performance no longer a headline weakness.
- The crates.io story published and exercised by a real consumer.
