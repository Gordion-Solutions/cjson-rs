#!/bin/sh
# Lay out built cjson-rs files as an upstream-cJSON-style install.
# Used by install.sh (after a cargo build) and shipped in release archives
# as install.sh (no Rust needed).
#
# usage: install-files.sh <libdir> <includedir> <packagingdir> <prefix> [DESTDIR]
#   <libdir> holds libcjson.{so,dylib} and libcjson.a as cargo names them.
set -e
libdir=${1:?usage: install-files.sh <libdir> <includedir> <packagingdir> <prefix> [DESTDIR]}
incdir=${2:?}; pkgdir=${3:?}; prefix=${4:?}; destdir=${5:-}
CJSON_ABI_VERSION=1.7.19
d="$destdir$prefix"
mkdir -p "$d/include/cjson" "$d/lib/pkgconfig" "$d/lib/cmake/cJSON"
install -m 644 "$incdir/cjson/cJSON.h" "$d/include/cjson/cJSON.h"
install -m 644 "$incdir/cjson/cJSON_Utils.h" "$d/include/cjson/cJSON_Utils.h"
install -m 644 "$libdir/libcjson.a" "$d/lib/libcjson.a"
case $(uname) in
  Darwin)
    install -m 755 "$libdir/libcjson.dylib" "$d/lib/libcjson.$CJSON_ABI_VERSION.dylib"
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
    install -m 755 "$libdir/libcjson.so" "$d/lib/libcjson.so.$CJSON_ABI_VERSION"
    ln -sf "libcjson.so.$CJSON_ABI_VERSION" "$d/lib/libcjson.so.1"
    ln -sf libcjson.so.1 "$d/lib/libcjson.so"
    ln -sf libcjson.so.1 "$d/lib/libcjson_utils.so.1"
    ln -sf libcjson_utils.so.1 "$d/lib/libcjson_utils.so"
    private_libs="-lm -ldl -lpthread"
    ;;
esac
sed -e "s|@PREFIX@|$prefix|" -e "s|@VERSION@|$CJSON_ABI_VERSION|" \
    -e "s|@PRIVATE_LIBS@|$private_libs|" \
    "$pkgdir/libcjson.pc.in" > "$d/lib/pkgconfig/libcjson.pc"
sed -e "s|@PREFIX@|$prefix|" -e "s|@VERSION@|$CJSON_ABI_VERSION|" \
    "$pkgdir/libcjson_utils.pc.in" > "$d/lib/pkgconfig/libcjson_utils.pc"
sed -e "s|@VERSION@|$CJSON_ABI_VERSION|" \
    "$pkgdir/cJSONConfigVersion.cmake.in" > "$d/lib/cmake/cJSON/cJSONConfigVersion.cmake"
install -m 644 "$pkgdir/cJSONConfig.cmake" "$d/lib/cmake/cJSON/cJSONConfig.cmake"
echo "installed cjson-safe as libcjson $CJSON_ABI_VERSION (with cJSON_Utils) under $d"
