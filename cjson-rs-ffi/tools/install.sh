#!/bin/sh
# Build cjson-rs and install it as a drop-in libcjson under <prefix>, laid out
# like upstream cJSON's `make install` / `cmake --install`:
#   <prefix>/include/cjson/cJSON.h
#   <prefix>/lib/libcjson.so.1.7.19, libcjson.so.1, libcjson.so   (Linux)
#   <prefix>/lib/libcjson.1.7.19.dylib, libcjson.1.dylib, libcjson.dylib (macOS)
#   <prefix>/lib/libcjson.a
#   <prefix>/lib/pkgconfig/libcjson.pc
#   <prefix>/lib/cmake/cJSON/{cJSONConfig,cJSONConfigVersion}.cmake
#
# usage: install.sh <prefix> [DESTDIR]
# The pkg-config and CMake files report the cJSON release whose ABI this
# library implements (1.7.19), not cjson-rs's own version, so
# `pkg-config --atleast-version=1.7.0 libcjson` passes.
set -e
prefix=${1:?usage: install.sh <prefix> [DESTDIR]}
here=$(cd "$(dirname "$0")" && pwd)
ffi=$(cd "$here/.." && pwd)
root=$(cd "$ffi/.." && pwd)
(cd "$root" && cargo build -q --release -p cjson-rs-ffi)
exec sh "$here/install-files.sh" "$root/target/release" "$ffi/include" "$ffi/packaging" "$prefix" "${2:-}"
