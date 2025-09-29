#!/bin/bash

set -exo pipefail

# export FEATURES="non-fips"

if [ -z "$TARGET" ]; then
  echo "Error: TARGET is not set. Examples of TARGET are x86_64-unknown-linux-gnu, x86_64-apple-darwin, aarch64-apple-darwin."
  exit 1
fi

if [ "$DEBUG_OR_RELEASE" = "release" ]; then
  RELEASE="--release"
fi

if [ -n "$FEATURES" ]; then
  FEATURES="--features $FEATURES"
fi

if [ -z "$FEATURES" ]; then
  echo "Info: FEATURES is not set."
  unset FEATURES
fi

if [ -z "$OPENSSL_DIR" ]; then
  echo "Error: OPENSSL_DIR is not set. Example OPENSSL_DIR=/usr/local/openssl"
  exit 1
fi

export RUST_LOG="cosmian_findex_cli=error,cosmian_findex_server=error,cosmian_findex_client=debug"

# shellcheck disable=SC2086
cargo test --workspace --bins --target $TARGET $RELEASE $FEATURES

# shellcheck disable=SC2086
cargo bench --target $TARGET $FEATURES --no-run

echo "SQLite is running on filesystem"
# shellcheck disable=SC2086
FINDEX_TEST_DB="sqlite-findex" cargo test --workspace --lib --target $TARGET $RELEASE $FEATURES -- --nocapture

if nc -z "$REDIS_HOST" "$REDIS_PORT"; then
  echo "Redis is running at $REDIS_HOST:$REDIS_PORT"
  # shellcheck disable=SC2086
  FINDEX_TEST_DB="redis-findex" cargo test --workspace --lib --target $TARGET $RELEASE $FEATURES -- --nocapture
else
  echo "Redis is not running at $REDIS_HOST:$REDIS_PORT"
fi
