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

23 Oct 2026, Intel Core i5-7600K 3.8 GHz (4 cores), macOS 13.7.2, Apple clang
14.0.3 (`-O2`), Rust 1.99, cjson-rs 0.2.0 + raw-print fix, cJSON 1.7.19 built
as Release. Throughput in MB/s of input; higher is better.

| Document | Operation | cJSON | cjson-rs | cjson-rs vs cJSON |
|---|---|---:|---:|---:|
| records.json (41 MB) | parse | 114.7 | 17.6 | 0.15x |
| | print | 132.9 | 155.2 | 1.17x |
| | print, formatted | 123.3 | 152.5 | 1.24x |
| numbers.json (31 MB) | parse | 72.2 | 43.2 | 0.60x |
| | print | 20.9 | 77.0 | 3.7x |
| | print, formatted | 20.7 | 79.2 | 3.8x |
| strings.json (36 MB) | parse | 357.0 | 81.4 | 0.23x |
| | print | 249.2 | 181.8 | 0.73x |
| | print, formatted | 247.7 | 193.0 | 0.78x |
| nested.json (8 MB) | parse | 41.5 | 5.4 | 0.13x |
| | print | 100.8 | 177.5 | 1.76x |
| | print, formatted | 100.4 | 205.2 | 2.04x |

Delete (freeing the tree) is the same within noise on every document.

**Parsing is slower than cJSON, by 1.7x to 7.6x.** Printing is faster on three
of four documents. The strings document prints slower (0.73x).

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
