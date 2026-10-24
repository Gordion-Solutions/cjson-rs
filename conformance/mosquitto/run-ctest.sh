echo /opt/cjson-$P/lib > /etc/ld.so.conf.d/cjson.conf && ldconfig
ldconfig -p | grep libcjson
ldd /out/mosq-$V/src/mosquitto | grep cjson
git -C /mosq clean -fdq 2>/dev/null
cd /out/mosq-$V
ctest --timeout 120 --output-on-failure -j1 > /out/ctest-$V.log 2>&1
tail -5 /out/ctest-$V.log
