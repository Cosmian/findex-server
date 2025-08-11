#!/bin/bash

set -ex

# Config paths
CONFIG=~/.cosmian/cosmian-no-tls.toml
TLS_CONFIG=~/.cosmian/cosmian-tls.toml
URL_HTTP="http://0.0.0.0:6668"
URL_HTTPS="https://0.0.0.0:6669"
CLI_VERSION="1.2.0"

# Cert paths
CA_CERT="test_data/client_server/ca/ca.crt"
CLIENT_CERT="test_data/client_server/owner/owner.client.acme.com.crt"
CLIENT_KEY="test_data/client_server/owner/owner.client.acme.com.key"
CLIENT_PKCS12_PATH="test_data/client_server/owner/owner.client.acme.com.p12"


# install cli
sudo apt update && sudo apt install -y wget
wget "https://package.cosmian.com/cli/$CLI_VERSION/ubuntu-24.04/cosmian-cli_$CLI_VERSION-1_amd64.deb"
sudo apt install ./"cosmian-cli_$CLI_VERSION-1_amd64.deb"
cosmian --version

# update cli conf
mkdir -p ~/.cosmian
touch $CONFIG $TLS_CONFIG

echo '
[kms_config.http_config]
server_url = "http://0.0.0.0:9998"

[findex_config.http_config]
server_url = "'$URL_HTTP'"
' | tee $CONFIG

echo '
[kms_config.http_config]
server_url = "http://0.0.0.0:9998"

[findex_config.http_config]
server_url = "'$URL_HTTPS'"
accept_invalid_certs = true
ssl_client_pkcs12_path = "'$CLIENT_PKCS12_PATH'"
ssl_client_pkcs12_password = "password"
' | tee $TLS_CONFIG

# Run docker containers
docker compose -f .github/scripts/docker-compose-authentication-tests.yml up -d --wait

# Wait for the containers to be ready
sleep 20

# Function to test OpenSSL connections
openssl_test() {
  local host_port=$1
  local tls_version=$2
  echo "Testing $host_port with TLS $tls_version"
  openssl s_client -showcerts -debug -"$tls_version" -connect "$host_port" \
    -CAfile "$CA_CERT" \
    -cert "$CLIENT_CERT" \
    -key "$CLIENT_KEY"
}

# Display server version
cosmian -c "$CONFIG" findex server-version
cosmian -c "$TLS_CONFIG" findex server-version

# Test TLS HTTPS server
openssl_test "127.0.0.1:6669" "tls1_2"
# openssl_test "127.0.0.1:6669" "tls1_3"
