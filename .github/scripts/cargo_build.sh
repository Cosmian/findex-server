#!/bin/bash

set -exo pipefail

if [ -z "$TARGET" ]; then
  echo "Error: TARGET is not set. Examples of TARGET are x86_64-unknown-linux-gnu, x86_64-apple-darwin, aarch64-apple-darwin."
  exit 1
fi

if [ "$DEBUG_OR_RELEASE" = "release" ]; then
  RELEASE="--release"
fi

if [ -z "$OPENSSL_DIR" ]; then
  echo "Error: OPENSSL_DIR is not set. Example OPENSSL_DIR=/usr/local/openssl"
  exit 1
fi

rustup target add "$TARGET"

# shellcheck disable=SC2086
cargo build -p cosmian_findex_server --target $TARGET $RELEASE

COSMIAN_FINDEX_SERVER_EXE="target/$TARGET/$DEBUG_OR_RELEASE/cosmian_findex_server"

# Test binary functionality
."/$COSMIAN_FINDEX_SERVER_EXE" --help

# Check for dynamic OpenSSL linkage
if [ "$(uname)" = "Linux" ]; then
  LDD_OUTPUT_SERVER=$(ldd "$COSMIAN_FINDEX_SERVER_EXE")
  echo "Server LDD output: $LDD_OUTPUT_SERVER"
  if echo "$LDD_OUTPUT_SERVER" | grep -qi ssl; then
    echo "Error: Dynamic OpenSSL linkage detected on Linux (ldd | grep ssl)."
    exit 1
  fi
else
  OTOOL_OUTPUT_SERVER=$(otool -L "$COSMIAN_FINDEX_SERVER_EXE")
  echo "Server otool output: $OTOOL_OUTPUT_SERVER"
  if echo "$OTOOL_OUTPUT_SERVER" | grep -qi ssl; then
    echo "Error: Dynamic OpenSSL linkage detected on macOS (otool -L | grep openssl)."
    exit 1
  fi
fi
