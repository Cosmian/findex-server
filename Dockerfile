FROM rust:1.85.0-bullseye AS builder

LABEL version="0.4.14"
LABEL name="Cosmian Findex server docker container"

ENV DEBIAN_FRONTEND=noninteractive

WORKDIR /root/findex-server

COPY . /root/findex-server

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
