# Mosquitto against cjson-rs

[Eclipse Mosquitto](https://github.com/eclipse-mosquitto/mosquitto) v2.1.2 uses
cJSON in the broker (HTTP API, control), the dynamic-security and SQLite
persistence plugins, `mosquitto_ctrl`, `mosquitto_sub`'s JSON output and its
shared common library. It reads struct fields (`valueint`, `valuestring`)
directly, so it exercises the binary layout as well as the functions.

Every cJSON function Mosquitto calls is one of cjson-rs's 78 exports. Three
names that look like cJSON's (`cJSON_AddIntToObject`, `cJSON_AddUIntToObject`,
`cJSON_CreateInt`) are Mosquitto's own helpers in `common/json_help.c`.

Mosquitto finds cJSON through its own `FindcJSON.cmake` (the header
`cjson/cJSON.h` and a library called `cjson`), not through pkg-config or CMake
package files. cjson-rs's install layout satisfies it unchanged.

## Result

Mosquitto v2.1.2, Ubuntu 22.04 x86_64, its 404 registered tests (`ctest`), built
twice: once against upstream cJSON 1.7.19 and once against cjson-rs. `ldd`
confirms each broker loaded the intended `libcjson.so.1`.

| Build | Passed | Failed |
|---|---:|---:|
| upstream cJSON 1.7.19 | 395 | 9 |
| cjson-rs, before the raw-print fix | 379 | 25 |
| cjson-rs, after the raw-print fix | 395 | 9 |

### Differences found and why

- **17 dynamic-security tests failed with cjson-rs and passed with cJSON.**
  Cause: `cJSON_AddRawToObject` and `cJSON_CreateRaw` items were printed as
  quoted strings by `cJSON_PrintUnformatted` and `cJSON_PrintPreallocated`, so
  `{"count":1}` went out as `{"count":"1"}`. Mosquitto builds integers that way,
  and the broker then ignored the count. Fixed in cjson-rs; regression tests in
  `cjson-rs-ffi/tests/ffi_smoke.rs`. After the fix all 17 pass.
- **`client-02-subscribe-format`** fails intermittently on both libraries
  (3 of 8 runs pass on cJSON, 4 of 8 on cjson-rs). The test starts
  `mosquitto_sub` and publishes after a fixed 0.1 s, which loses the race on a
  loaded machine. Not a cJSON difference.
- **`broker-15-persist-client-expired-session-persist_sqlite`** and
  **`client-02-subscribe-env`** each failed on one library in one run and not in
  another. Flaky.

### Failures that are the same with both libraries

| Test | Reason |
|---|---|
| `broker-01-bad-initial-packets` | Needs the Python module `psutil`, missing from the image |
| `broker-22-http-api-*` (5) | The CMake build has no HTTP API option; see below |
| `lib-08-ssl-fake-cacert` | Not related to cJSON; not investigated further |

The HTTP API uses cJSON, so it was run separately with Mosquitto's Makefile
build (`WITH_HTTP_API=yes`, `libmicrohttpd`): all five `22-http-api-*` tests pass
with both upstream cJSON and cjson-rs.

## Not covered

Linux x86_64 only. Not run: macOS, Linux arm64, Mosquitto's `fuzzing` targets.

## Reproducing

The scripts are the ones used, written for a Docker setup where the repository
is mounted at `/repo`, a Mosquitto checkout at `/mosq` and a scratch directory
at `/out`: `Dockerfile` (build image), `install-libs.sh` (builds both
libraries), `build-mosquitto.sh` (builds Mosquitto against each),
`run-ctest.sh` (runs the suite; set `V` and `P` to `upstream` or `rs`) and
`run-http-api.sh`. They contain no secrets and have not been tidied into a
single command.
