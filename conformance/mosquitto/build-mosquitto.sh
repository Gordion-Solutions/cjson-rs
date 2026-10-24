set -e
mkdir -p /work /out
# clean copies; never write into the mounted repos
tar -C /repo --exclude=target --exclude=.git -cf - . | tar -C /work -xf -
# --- baseline: upstream cJSON 1.7.19 from the pinned submodule
cmake -S /repo/conformance/upstream-tests -B /out/up-build -DENABLE_CJSON_TEST=OFF -DENABLE_CJSON_UTILS=ON -DBUILD_SHARED_AND_STATIC_LIBS=OFF -DCMAKE_INSTALL_PREFIX=/opt/cjson-upstream -DCMAKE_BUILD_TYPE=Release >/dev/null
cmake --build /out/up-build -j4 >/dev/null && cmake --install /out/up-build >/dev/null
# --- ours
sh /work/cjson-rs-ffi/tools/install.sh /opt/cjson-rs >/dev/null
for v in upstream rs; do
  p=/opt/cjson-$v; [ $v = rs ] && p=/opt/cjson-rs
  rm -rf /out/mosq-$v && mkdir -p /out/mosq-$v; export CPATH=$p/include LIBRARY_PATH=$p/lib
  cmake -S /mosq -B /out/mosq-$v -DCMAKE_PREFIX_PATH=$p -DCMAKE_BUILD_TYPE=Release \
        -DWITH_DOCS=OFF -DWITH_TESTS=ON -DWITH_LTO=OFF >/out/mosq-$v/cmake.log 2>&1 || { tail -20 /out/mosq-$v/cmake.log; exit 1; }
  cmake --build /out/mosq-$v -j4 >/out/mosq-$v/build.log 2>&1 || { tail -30 /out/mosq-$v/build.log; exit 1; }
  echo "== built mosquitto against $v"
  LD_LIBRARY_PATH=$p/lib ldd /out/mosq-$v/src/mosquitto | grep -i cjson
done
