FROM debian:trixie

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        build-essential \
        pkg-config \
        ca-certificates \
        curl \
        git \
        pkg-config

# Optional: install additional native dependencies here
# RUN apt-get update && apt-get install -y libssl-dev ...

RUN apt-get install -y \
    cmake \
    crossbuild-essential-arm64

RUN dpkg --add-architecture arm64
RUN apt-get update


RUN ln -s /usr/bin/aarch64-linux-gnu-gcc /usr/local/bin/aarch64-unknown-linux-gnu-gcc && \
    ln -s /usr/bin/aarch64-linux-gnu-g++ /usr/local/bin/aarch64-unknown-linux-gnu-g++ && \
    ln -s /usr/bin/aarch64-linux-gnu-gcc /usr/local/bin/aarch64-unknown-linux-gnu-cc && \
    ln -s /usr/bin/aarch64-linux-gnu-g++ /usr/local/bin/aarch64-unknown-linux-gnu-c++

RUN apt-get install -y libheif-dev:arm64 libsharpyuv-dev:arm64 libwebp-dev:arm64
ENV PKG_CONFIG_ALLOW_CROSS=1
ENV CARGO_HOME=/usr/local/cargo