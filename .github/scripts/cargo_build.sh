#!/bin/bash

set -ex

# --- Declare the following variables for tests
# export TARGET=x86_64-unknown-linux-gnu
# export DEBUG_OR_RELEASE=debug
# export SKIP_SERVICES_TESTS="--skip test_redis"

if [ -z "$TARGET" ]; then
  echo "Error: TARGET is not set."
  exit 1
fi

if [ "$DEBUG_OR_RELEASE" = "release" ]; then
  RELEASE="--release"
fi

if [ -z "$SKIP_SERVICES_TESTS" ]; then
  echo "Info: SKIP_SERVICES_TESTS is not set."
  unset SKIP_SERVICES_TESTS
fi

rustup target add "$TARGET"

# shellcheck disable=SC2086
cargo build --target $TARGET $RELEASE $FEATURES

# shellcheck disable=SC2086
cargo test --target $TARGET $RELEASE --workspace -- --nocapture $SKIP_SERVICES_TESTS
