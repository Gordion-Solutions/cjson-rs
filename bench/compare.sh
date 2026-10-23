#!/bin/sh
# Best-of-5 wall time for parse and print: upstream cJSON vs cjson-rs.
# The same C program (cbench.c) is linked against each library.
#
# usage: compare.sh <upstream-prefix> <cjson-rs-prefix> [data-dir]
#   prefixes hold include/cjson/cJSON.h and lib/libcjson.* (cjson-rs:
#   cjson-rs-ffi/tools/install.sh <prefix>; upstream: cmake --install)
#   data-dir comes from generate.py (default: bench-data)
set -e
up=${1:?usage: compare.sh <upstream-prefix> <cjson-rs-prefix> [data-dir]}
rs=${2:?usage}
data=${3:-bench-data}
here=$(cd "$(dirname "$0")" && pwd)
work=$(mktemp -d); trap 'rm -rf "$work"' EXIT
for v in up rs; do
  eval p=\$$v
  cc -O2 -I"$p/include/cjson" "$here/cbench.c" -L"$p/lib" -lcjson -Wl,-rpath,"$p/lib" -o "$work/cbench-$v"
done
for f in records numbers strings nested; do
  for v in up rs; do
    echo "### $f.json [$v]"; "$work/cbench-$v" "$data/$f.json"
  done
done
echo "### output identical to upstream?"
for f in records numbers strings nested; do
  for m in --dump --dump-pretty; do
    "$work/cbench-up" $m "$data/$f.json" > "$work/a"; "$work/cbench-rs" $m "$data/$f.json" > "$work/b"
    if cmp -s "$work/a" "$work/b"; then echo "$f $m: identical"; else echo "$f $m: DIFFERENT"; fi
  done
done
