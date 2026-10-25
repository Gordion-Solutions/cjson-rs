# Contributing to cjson-safe

Thanks for your interest. This project is a memory-safe drop-in for C's
[cJSON](https://github.com/DaveGamble/cJSON): it reimplements cJSON's API and
ABI in Rust so existing C/C++ consumers link against it unmodified. That goal
shapes what changes are in scope.

## Scope

- **Yes:** correctness and safety fixes, closer behavioural parity with upstream
  cJSON, performance, portability, better tests, and the `cJSON_Utils` surface.
- **Probably not:** new APIs that upstream cJSON does not have. If you want a
  general-purpose Rust JSON library, use
  [`serde_json`](https://crates.io/crates/serde_json) — this crate exists to
  replace C's libcjson safely, not to grow beyond it.

If you are unsure whether a change fits, open an issue first and ask.

## Repository layout

Two crates in one workspace:

- `cjson-rs/` — the safe core, `#![forbid(unsafe_code)]`. Published as
  `cjson-safe-core`.
- `cjson-rs-ffi/` — the C ABI shim; **all `unsafe` lives here**. Published as
  `cjson-safe`, and what produces `libcjson.{so,dylib,a}`.

See the **Layout** section of the [README](README.md) for a file-by-file map.

## Building and testing

```sh
# Build both crates -> target/release/libcjson.{so,dylib,a}
cargo build --release --workspace

# Rust tests
cargo test --workspace

# C smoke test (compiles a C program, links it against the built library)
bash cjson-rs-ffi/tests/c/run-c-smoke.sh

# Public-API subset of upstream cJSON's own test suite
bash conformance/run-upstream-tests.sh
```

The conformance suite uses a git submodule. Clone with `--recurse-submodules`,
or run `git submodule update --init` afterwards.

A change is not ready until `cargo test --workspace` and the C smoke test both
pass locally. CI runs the same checks on Linux and macOS.

## Safety expectations

This is the whole point of the project, so the bar is high:

- The core crate must stay `#![forbid(unsafe_code)]`. If a change needs
  `unsafe`, it belongs in `cjson-rs-ffi`, not `cjson-rs`.
- Any change to the FFI shim should keep it Miri-clean:

  ```sh
  cargo +nightly miri test --workspace --test ffi_smoke
  ```

- New parsing or FFI paths should come with a fuzz target, or extend an
  existing one:

  ```sh
  cd cjson-rs-ffi
  cargo +nightly fuzz run fuzz_parse -- -max_total_time=60
  cargo +nightly fuzz run fuzz_ffi_roundtrip -- -max_total_time=60
  ```

- Fix root causes, not symptoms. Don't paper over a failing test; find why it
  fails.

## ABI stability

C consumers depend on the exact struct layout, symbol names, and SONAME
(`libcjson.so.1` / `libcjson.1.dylib`). Do not change the exported symbol set,
the `cJSON` struct fields, or the soname without a very good reason and a note
in `CHANGELOG.md`. CI checks that only `cJSON_*` / `cJSONUtils_*` symbols are
exported.

## Pull requests

- Keep commits focused; write a clear commit message explaining *why*.
- Update `CHANGELOG.md` for anything user-visible.
- Reference the issue a PR addresses, if there is one.
- Expect review to focus on safety and parity with upstream cJSON.

## Security

Please do **not** open a public issue for a security vulnerability. See
[SECURITY.md](SECURITY.md) for how to report privately.

## Licence

By contributing you agree your contributions are licensed under the project's
[MIT licence](LICENSE), the same licence as upstream cJSON.
