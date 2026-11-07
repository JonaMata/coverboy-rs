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
    ln -s /usr/bin/aarch64-linux-gnu-cc /usr/local/bin/aarch64-unknown-linux-gnu-cc && \
    ln -s /usr/bin/aarch64-linux-gnu-c++ /usr/local/bin/aarch64-unknown-linux-gnu-c++ && \
    ln -s /usr/bin/aarch64-linux-gnu-strip /usr/local/bin/aarch64-unknown-linux-gnu-strip && \
    ln -s /usr/bin/aarch64-linux-gnu-objcopy /usr/local/bin/aarch64-unknown-linux-gnu-objcopy

RUN apt-get install -y libheif-dev:arm64 libsharpyuv-dev:arm64 libwebp-dev:arm64
ENV PKG_CONFIG_ALLOW_CROSS=1
ENV CARGO_HOME=/usr/local/cargo
ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-unknown-linux-gnu-gcc
ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_STRIP=aarch64-unknown-linux-gnu-strip
ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_OBJCOPY=aarch64-unknown-linux-gnu-objcopy
ENV BINDGEN_EXTRA_CLANG_ARGS="--target=aarch64-linux-gnu"