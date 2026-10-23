#!/bin/sh
# Build a release archive cjson-rs-<version>-<label>.tar.gz (plus .sha256).
# It holds the built library, the header, the pkg-config/CMake templates and
# an install.sh that lays them out under any prefix without a Rust toolchain.
#
# usage: package.sh <outdir> [label]    (label defaults to <os>-<arch>)
set -e
outdir=${1:?usage: package.sh <outdir> [label]}
here=$(cd "$(dirname "$0")" && pwd)
ffi=$(cd "$here/.." && pwd)
root=$(cd "$ffi/.." && pwd)
os=$(uname | tr '[:upper:]' '[:lower:]'); [ "$os" = darwin ] && os=macos
label=${2:-$os-$(uname -m)}
version=$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/Cargo.toml" | head -1)
name="cjson-rs-$version-$label"
mkdir -p "$outdir"; outdir=$(cd "$outdir" && pwd)
(cd "$root" && cargo build -q --release -p cjson-rs-ffi)
stage=$(mktemp -d); trap 'rm -rf "$stage"' EXIT
p="$stage/$name"
mkdir -p "$p/lib" "$p/include/cjson" "$p/packaging" "$p/tools"
rel="$root/target/release"
cp "$rel/libcjson.a" "$p/lib/"
cp "$rel"/libcjson.dylib "$p/lib/" 2>/dev/null || cp "$rel/libcjson.so" "$p/lib/"
cp "$ffi/include/cjson/cJSON.h" "$ffi/include/cjson/LICENSE.cJSON" "$p/include/cjson/"
cp "$ffi"/packaging/* "$p/packaging/"
cp "$root/LICENSE" "$p/LICENSE"
cp "$here/install-files.sh" "$p/tools/install-files.sh"
cat > "$p/install.sh" <<'INST'
#!/bin/sh
# usage: install.sh <prefix> [DESTDIR]
set -e
prefix=${1:?usage: install.sh <prefix> [DESTDIR]}
here=$(cd "$(dirname "$0")" && pwd)
exec sh "$here/tools/install-files.sh" "$here/lib" "$here/include" "$here/packaging" "$prefix" "${2:-}"
INST
chmod +x "$p/install.sh"
cat > "$p/README.txt" <<TXT
cjson-rs $version ($label): memory-safe drop-in replacement for cJSON $(sed -n 's/^CJSON_ABI_VERSION=//p' "$here/install-files.sh").

  ./install.sh /usr/local        (sudo if needed; any prefix works)
  cc prog.c \$(pkg-config --cflags --libs libcjson)

https://github.com/Gordion-Solutions/cjson-rs
TXT
tar -C "$stage" -czf "$outdir/$name.tar.gz" "$name"
(cd "$outdir" && { shasum -a 256 "$name.tar.gz" 2>/dev/null || sha256sum "$name.tar.gz"; } > "$name.tar.gz.sha256")
echo "$outdir/$name.tar.gz"
