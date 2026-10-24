# Benchmarks: cjson-rs vs upstream cJSON

Best-of-5 wall time for parse, print (unformatted and formatted) and delete.
One C program (`cbench.c`) is linked against each library, so both run
identical code on identical files.

```sh
python3 bench/generate.py bench-data
cmake -S conformance/upstream-tests -B /tmp/up-build -DCMAKE_BUILD_TYPE=Release \
      -DENABLE_CJSON_TEST=OFF -DENABLE_CJSON_UTILS=OFF -DCMAKE_INSTALL_PREFIX=/tmp/up
cmake --build /tmp/up-build && cmake --install /tmp/up-build
cjson-rs-ffi/tools/install.sh /tmp/rs
bench/compare.sh /tmp/up /tmp/rs bench-data
```

## Results

24 Oct 2026, Intel Core i5-7600K 3.8 GHz (4 cores), macOS 13.7.2, Apple clang
14.0.3 (`-O2`), Rust 1.99, cjson-safe 0.3.0 (streaming tokeniser + single-copy
string runs), cJSON 1.7.19 built as Release. Throughput in MB/s of input;
higher is better; `×` is cjson-safe relative to cJSON.

| Document | Operation | cJSON | cjson-safe | × |
|---|---|---:|---:|---:|
| records.json (43 MB) | parse | 117.6 | 26.6 | 0.23× |
| | print | 129.0 | 180.9 | 1.40× |
| | print, formatted | 122.4 | 154.7 | 1.26× |
| numbers.json (32 MB) | parse | 72.9 | 58.4 | 0.80× |
| | print | 19.7 | 75.0 | 3.8× |
| | print, formatted | 22.1 | 71.0 | 3.2× |
| strings.json (38 MB) | parse | 390.1 | 147.6 | 0.38× |
| | print | 244.5 | 225.0 | 0.92× |
| | print, formatted | 240.7 | 234.6 | 0.97× |
| nested.json (8 MB) | parse | 41.6 | 8.1 | 0.19× |
| | print | 97.9 | 179.1 | 1.83× |
| | print, formatted | 97.7 | 182.5 | 1.87× |

Delete (freeing the tree) is the same within noise on every document.

**Parse is slower than cJSON — from 1.25× (numbers) to ~5× (nested/objects).**
The cause is the intermediate Rust `Value` tree built before the `cJSON` tree;
removing it (parsing straight into `cJSON` nodes) is tracked on the roadmap.
**Print is faster** on three of four documents — up to 3.8× on number-heavy
data, where cJSON's `%1.15g` formatting with a round-trip re-check is
expensive — and at parity on strings.

## Output compared with cJSON

Printed output (unformatted and formatted) is byte-identical to cJSON's on all
four documents. It is not identical for every number: very large and very small
values, `-0`, and doubles that cJSON rounds, print differently.

| Value | cJSON 1.7.19 | cjson-rs |
|---|---|---|
| `1e20` | `1e+20` | `100000000000000000000` |
| `1.5e300` | `1.5e+300` | 301 digits |
| `-0` | `0` | `-0` |
| `0.30000000000000004` | `0.3` | `0.30000000000000004` |
| `123456789012345678` | `1.2345678901234568e+17` | `123456789012345680` |

Both forms are valid JSON and parse back to the same double, except where cJSON
rounds first (`0.3`).
