#!/bin/sh
# Check a release archive as a user would: unpack it, run its install.sh into
# a scratch prefix (no Rust involved), build a C program through pkg-config,
# and confirm it loaded the installed libcjson.1.
# usage: run-archive-test.sh <archive.tar.gz>
set -e
archive=$(cd "$(dirname "${1:?usage: run-archive-test.sh <archive.tar.gz>}")" && pwd)/$(basename "$1")
here=$(cd "$(dirname "$0")" && pwd)
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
tar -C "$tmp" -xzf "$archive"
"$tmp"/cjson-rs-*/install.sh "$tmp/prefix" >/dev/null
export PKG_CONFIG_PATH="$tmp/prefix/lib/pkgconfig"
pkg-config --atleast-version=1.7.0 libcjson
cc -Wall -Wextra -Werror "$here/install-probe.c" $(pkg-config --cflags --libs libcjson) \
   -Wl,-rpath,"$tmp/prefix/lib" -o "$tmp/probe"
"$tmp/probe" >/dev/null
case $(uname) in
  Darwin) lib=$(otool -L "$tmp/probe" | awk '/libcjson/ {print $1}') ;;
  *)      lib=$(ldd "$tmp/probe" | awk '/libcjson/ {print $3}') ;;
esac
case $lib in
  "$tmp"/prefix/lib/libcjson.*) echo "archive: ok ($lib)" ;;
  *) echo "archive: probe loaded '$lib', expected a library under $tmp/prefix" >&2; exit 1 ;;
esac
