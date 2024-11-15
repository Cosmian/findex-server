FROM ubuntu:22.04 AS builder

LABEL version="0.1.0"
LABEL name="Cosmian Findex Server docker container"

ENV DEBIAN_FRONTEND=noninteractive

WORKDIR /root

RUN apt-get update \
    && apt-get install --no-install-recommends -qq -y \
    curl \
    build-essential \
    libssl-dev \
    ca-certificates \
    libclang-dev \
    libsodium-dev \
    pkg-config \
    git \
    wget \
    && apt-get -y -q upgrade \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain "nightly-x86_64-unknown-linux-gnu"

COPY . /root/findex-server

WORKDIR /root/findex-server

RUN /root/.cargo/bin/cargo build --release --no-default-features

#
# Findex Server
#
FROM ubuntu:22.04 AS findex-server

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
    && apt-get install --no-install-recommends -qq -y \
    ca-certificates \
    libssl-dev \
    libsodium-dev \
    && apt-get -y -q upgrade \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /root/findex-server/target/release/cosmian_findex_server  /usr/bin/cosmian_findex_server
COPY --from=builder /root/findex-server/target/release/cosmian_findex_cli     /usr/bin/cosmian_findex_cli

#
# Create working directory
#
WORKDIR /data

EXPOSE 9998

ENTRYPOINT ["cosmian_findex_server"]
