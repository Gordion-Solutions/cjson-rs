set -e
mkdir -p /work && tar -C /repo --exclude=target --exclude=.git -cf - . | tar -C /work -xf -
cmake -S /repo/conformance/upstream-tests -B /tmp/up -DENABLE_CJSON_TEST=OFF -DENABLE_CJSON_UTILS=ON -DBUILD_SHARED_AND_STATIC_LIBS=OFF -DCMAKE_INSTALL_PREFIX=/opt/cjson-upstream -DCMAKE_BUILD_TYPE=Release >/dev/null
cmake --build /tmp/up -j4 >/dev/null && DESTDIR=/out/dest-upstream cmake --install /tmp/up >/dev/null
sh /work/cjson-rs-ffi/tools/install.sh /opt/cjson-rs /out/dest-rs >/dev/null
ls /out/dest-upstream/opt/cjson-upstream/lib /out/dest-rs/opt/cjson-rs/lib
