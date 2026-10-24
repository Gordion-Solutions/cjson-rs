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

Mosquitto v2.1.2, Ubuntu 22.04 x86_64, its 404 registered tests (`ctest`). The
same Mosquitto build was run twice, once loading upstream cJSON 1.7.19 and once
loading cjson-rs (`ldd` confirms which `libcjson.so.1` each run loaded).

| cJSON library | Passed | Failed |
|---|---:|---:|
| upstream cJSON 1.7.19 | 399 | 5 |
| cjson-rs, before the raw-print fix | 379 | 25 |
| cjson-rs, after the raw-print fix | 399 | 5 |

The 5 failures are the same with both libraries: the `broker-22-http-api-*`
tests, which the CMake build cannot build (it has no HTTP API option). They were
run separately with Mosquitto's Makefile build (`WITH_HTTP_API=yes`,
`libmicrohttpd`), and all five pass with both upstream cJSON and cjson-rs.

The 399 need two fixes to the test setup, neither of which touches cJSON (see
below): the Python module `psutil` in the image, and two small patches to
Mosquitto's own tests in `patches/`.

### Differences found and why

- **17 dynamic-security tests failed with cjson-rs and passed with cJSON.**
  Cause: `cJSON_AddRawToObject` and `cJSON_CreateRaw` items were printed as
  quoted strings by `cJSON_PrintUnformatted` and `cJSON_PrintPreallocated`, so
  `{"count":1}` went out as `{"count":"1"}`. Mosquitto builds integers that way,
  and the broker then ignored the count. Fixed in cjson-rs; regression tests in
  `cjson-rs-ffi/tests/ffi_smoke.rs`. After the fix all 17 pass.

### Test-setup problems (same on both libraries)

| Test | Cause | Fix |
|---|---|---|
| `broker-01-bad-initial-packets` | The image lacked the Python module `psutil` | Installed in the `Dockerfile` |
| `client-02-subscribe-format` | Race in the test: it starts `mosquitto_sub`, sleeps a fixed 0.1 s, then publishes once at QoS 0, which is lost if the subscriber has not subscribed yet. Failed 12 of 60 runs on upstream and 11 of 60 on cjson-rs | `patches/02-subscribe-format-no-race.patch` republishes until the subscriber (`-C 1`) has exited; the formatting checks are unchanged. 30 of 30 on both |
| `broker-15-persist-client-expired-session-persist_sqlite` | Race in the test: it closes the client socket and terminates the broker straight away, so the client row may be persisted before the disconnect is processed. Failed 10 of 10 on upstream in one batch of runs | `patches/15-persist-expired-session-no-race.patch` waits 0.5 s after the close; the checks are unchanged. 10 of 10 on both |
| `lib-08-ssl-fake-cacert` | Passed alone on both libraries; failed once in a full run on upstream. Cause not found | None needed: passes in the full runs above. Not a cJSON difference |

## Not covered

Linux x86_64 only. Not run: macOS, Linux arm64, Mosquitto's `fuzzing` targets.

## Reproducing

The scripts are the ones used, written for a Docker setup where the repository
is mounted at `/repo`, a Mosquitto checkout at `/mosq` and a scratch directory
at `/out`: `Dockerfile` (build image), `install-libs.sh` (builds both
libraries), `build-mosquitto.sh` (builds Mosquitto against each),
`run-ctest.sh` (runs the suite; set `V` and `P` to `upstream` or `rs`) and
`run-http-api.sh`. Apply `patches/*.patch` to the Mosquitto checkout
(`git apply`) before running the suite. They contain no secrets and have not been tidied into a
single command.
