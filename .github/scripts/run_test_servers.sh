#!/bin/sh

set -ex

docker-compose up -d

cargo run --bin cosmian_findex_server -- --https-p12-file ~/Cosmian/github/cli/test_data/certificates/client_server/server/kmserver.acme.com.p12 --https-p12-password password --authority-cert-file ~/Cosmian/github/cli/test_data/certificates/client_server/server/ca.crt --port 6660 &

cargo run --bin cosmian_findex_server

# To stop the servers:
# killall cosmian_findex_server
