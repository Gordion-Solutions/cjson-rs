#!/bin/sh
# Install into a scratch prefix, then build a C program against it through
# pkg-config and through CMake's find_package(cJSON), run both, and check
# each linked the installed libcjson.1 (not a system cJSON).
set -e
here=$(cd "$(dirname "$0")" && pwd)
ffi=$(cd "$here/../.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
"$ffi/tools/install.sh" "$tmp/prefix" >/dev/null

linked() { # <binary>: print the libcjson it loads
  case $(uname) in
    Darwin) otool -L "$1" | awk '/libcjson/ {print $1}' ;;
    *)      ldd "$1" | awk '/libcjson/ {print $3}' ;;
  esac
}
check() { # <binary> <label>
  lib=$(linked "$1")
  case $lib in
    "$tmp"/prefix/lib/libcjson.*) echo "$2: ok ($lib)" ;;
    *) echo "$2: loaded '$lib', expected a library under $tmp/prefix" >&2; exit 1 ;;
  esac
}

PKG_CONFIG_PATH="$tmp/prefix/lib/pkgconfig"; export PKG_CONFIG_PATH
pkg-config --atleast-version=1.7.0 libcjson
cc -Wall -Wextra -Werror "$here/install-probe.c" $(pkg-config --cflags --libs libcjson) \
   -Wl,-rpath,"$tmp/prefix/lib" -o "$tmp/probe_pc"
"$tmp/probe_pc" >/dev/null
check "$tmp/probe_pc" pkg-config

if command -v cmake >/dev/null 2>&1; then
  mkdir "$tmp/cm"
  cat > "$tmp/cm/CMakeLists.txt" <<CM
cmake_minimum_required(VERSION 3.10)
project(probe C)
find_package(cJSON 1.7 REQUIRED)
add_executable(probe $here/install-probe.c)
target_link_libraries(probe cJSON::cjson)
CM
  cmake -S "$tmp/cm" -B "$tmp/cm/b" -DCMAKE_PREFIX_PATH="$tmp/prefix" \
        -DCMAKE_BUILD_RPATH="$tmp/prefix/lib" >/dev/null
  cmake --build "$tmp/cm/b" >/dev/null
  "$tmp/cm/b/probe" >/dev/null
  check "$tmp/cm/b/probe" cmake
else
  echo "cmake: skipped (not installed)"
fi
