FROM ubuntu:22.04
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl git build-essential pkg-config cmake binutils \
    libssl-dev libsqlite3-dev libmicrohttpd-dev libedit-dev python3-psutil libcunit1-dev libgtest-dev libgmock-dev python3 python3-pip xsltproc docbook-xsl \
    && rm -rf /var/lib/apt/lists/*
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
ENV PATH=/root/.cargo/bin:$PATH
RUN cmake -S /usr/src/googletest -B /tmp/gt -DCMAKE_BUILD_TYPE=Release >/dev/null && cmake --build /tmp/gt -j4 >/dev/null && cmake --install /tmp/gt >/dev/null && rm -rf /tmp/gt
