#!/bin/sh
# Install cjson-rs as a drop-in libcjson under <prefix>, laid out like
# upstream cJSON's `make install` / `cmake --install`:
#   <prefix>/include/cjson/cJSON.h
#   <prefix>/lib/libcjson.so.1.7.19, libcjson.so.1, libcjson.so   (Linux)
#   <prefix>/lib/libcjson.1.7.19.dylib, libcjson.1.dylib, libcjson.dylib (macOS)
#   <prefix>/lib/libcjson.a
#   <prefix>/lib/pkgconfig/libcjson.pc
#   <prefix>/lib/cmake/cJSON/{cJSONConfig,cJSONConfigVersion}.cmake
#
# usage: install.sh <prefix> [DESTDIR]
# The pkg-config and CMake files report the cJSON release whose ABI this
# library implements (header and tests: CJSON_ABI_VERSION), not cjson-rs's
# own version, so `pkg-config --atleast-version=1.7.0 libcjson` passes.
set -e
prefix=${1:?usage: install.sh <prefix> [DESTDIR]}
destdir=${2:-}
CJSON_ABI_VERSION=1.7.19
here=$(cd "$(dirname "$0")" && pwd)
ffi=$(cd "$here/.." && pwd)
root=$(cd "$ffi/.." && pwd)
(cd "$root" && cargo build -q --release -p cjson-safe)
rel="$root/target/release"
d="$destdir$prefix"
mkdir -p "$d/include/cjson" "$d/lib/pkgconfig" "$d/lib/cmake/cJSON"
install -m 644 "$ffi/include/cjson/cJSON.h" "$d/include/cjson/cJSON.h"
install -m 644 "$ffi/include/cjson/cJSON_Utils.h" "$d/include/cjson/cJSON_Utils.h"
install -m 644 "$rel/libcjson.a" "$d/lib/libcjson.a"
case $(uname) in
  Darwin)
    install -m 755 "$rel/libcjson.dylib" "$d/lib/libcjson.$CJSON_ABI_VERSION.dylib"
    install_name_tool -id "$prefix/lib/libcjson.1.dylib" "$d/lib/libcjson.$CJSON_ABI_VERSION.dylib"
    ln -sf "libcjson.$CJSON_ABI_VERSION.dylib" "$d/lib/libcjson.1.dylib"
    ln -sf libcjson.1.dylib "$d/lib/libcjson.dylib"
    # cJSON_Utils lives in the same library; ship libcjson_utils as links to
    # it so -lcjson_utils / pkg-config / find_package(cJSON) cjson_utils all
    # resolve, with no second object loaded at runtime.
    ln -sf libcjson.1.dylib "$d/lib/libcjson_utils.1.dylib"
    ln -sf libcjson_utils.1.dylib "$d/lib/libcjson_utils.dylib"
    private_libs=
    ;;
  *)
    install -m 755 "$rel/libcjson.so" "$d/lib/libcjson.so.$CJSON_ABI_VERSION"
    ln -sf "libcjson.so.$CJSON_ABI_VERSION" "$d/lib/libcjson.so.1"
    ln -sf libcjson.so.1 "$d/lib/libcjson.so"
    ln -sf libcjson.so.1 "$d/lib/libcjson_utils.so.1"
    ln -sf libcjson_utils.so.1 "$d/lib/libcjson_utils.so"
    private_libs="-lm -ldl -lpthread"
    ;;
esac
sed -e "s|@PREFIX@|$prefix|" -e "s|@VERSION@|$CJSON_ABI_VERSION|" \
    -e "s|@PRIVATE_LIBS@|$private_libs|" \
    "$ffi/packaging/libcjson.pc.in" > "$d/lib/pkgconfig/libcjson.pc"
sed -e "s|@PREFIX@|$prefix|" -e "s|@VERSION@|$CJSON_ABI_VERSION|" \
    "$ffi/packaging/libcjson_utils.pc.in" > "$d/lib/pkgconfig/libcjson_utils.pc"
sed -e "s|@VERSION@|$CJSON_ABI_VERSION|" \
    "$ffi/packaging/cJSONConfigVersion.cmake.in" > "$d/lib/cmake/cJSON/cJSONConfigVersion.cmake"
install -m 644 "$ffi/packaging/cJSONConfig.cmake" "$d/lib/cmake/cJSON/cJSONConfig.cmake"
echo "installed cjson-rs as libcjson $CJSON_ABI_VERSION under $d"
