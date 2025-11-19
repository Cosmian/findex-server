FROM rust:1.85.0-bullseye AS builder

LABEL version="0.4.8"
LABEL name="Cosmian Findex server docker container"

ENV DEBIAN_FRONTEND=noninteractive

WORKDIR /root

RUN apt-get update \
    && apt-get install --no-install-recommends -qq -y \
    wget \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

COPY . /root/findex-server

WORKDIR /root/findex-server

RUN cargo build --release --no-default-features

#
# Findex server
#
FROM debian:bullseye-slim AS findex-server

COPY --from=builder /root/findex-server/target/release/cosmian_findex_server  /usr/bin/cosmian_findex_server

#
# Create working directory
#
WORKDIR /data

EXPOSE 6668

ENTRYPOINT ["cosmian_findex_server"]
