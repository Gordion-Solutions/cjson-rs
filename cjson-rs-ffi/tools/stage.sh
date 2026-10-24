#!/bin/sh
# Copy the built library to <dir> under cJSON's real file names
# (libcjson.1.dylib / libcjson.so.1), so that programs linked with
# -L<dir> -lcjson -Wl,-rpath,<dir> load it, not the system libcjson.
set -e
dir=${1:?usage: stage.sh <dir> [profile]}
profile=${2:-release}
root=$(cd "$(dirname "$0")/../.." && pwd)
# Build first: cargo test doesn't update target/<profile>/, so copying
# without building can stage a stale library.
if [ "$profile" = release ]; then flag=--release; else flag="--profile $profile"; fi
(cd "$root" && cargo build -q $flag -p cjson-safe)
mkdir -p "$dir"
case $(uname) in
  Darwin) cp "$root/target/$profile/libcjson.dylib" "$dir/libcjson.1.dylib"; ln -sf libcjson.1.dylib "$dir/libcjson.dylib" ;;
  *)      cp "$root/target/$profile/libcjson.so" "$dir/libcjson.so.1"; ln -sf libcjson.so.1 "$dir/libcjson.so" ;;
esac
