set -e
echo /opt/cjson/lib > /etc/ld.so.conf.d/cjson.conf && ldconfig
cd /mosq
git clean -fdxq 2>/dev/null || true
make -j4 WITH_DOCS=no WITH_TLS=yes WITH_HTTP_API=yes WITH_CTRL_SHELL=no WITH_TESTS=no \
  CFLAGS="-I/opt/cjson/include -O2" CPPFLAGS="-I/opt/cjson/include" LDFLAGS="-L/opt/cjson/lib" binary >/tmp/make.log 2>&1 || { tail -20 /tmp/make.log; exit 1; }
ldd src/mosquitto | grep -i -E "cjson|microhttpd"
cd test/broker
for t in 22-http-api-acl 22-http-api-api 22-http-api-auth 22-http-api-file 22-http-api-tls; do
  if python3 ./$t.py >/tmp/$t.log 2>&1; then echo "$V: $t PASS"; else echo "$V: $t FAIL: $(grep -v '^ ' /tmp/$t.log | tail -2 | tr '\n' ' ')"; fi
done
