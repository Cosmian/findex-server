#!/bin/bash

set -ex

# --- Declare the following variables for tests
# export TARGET=aarch64-apple-darwin
# export DEBUG_OR_RELEASE=debug
# export SKIP_SERVICES_TESTS="--skip test_findex --skip test_all_authentications --skip test_server_auth_matrix --skip test_datasets"
#

ROOT_FOLDER=$(pwd)

if [ "$DEBUG_OR_RELEASE" = "release" ]; then
  # First build the Debian and RPM packages.
  rm -rf target/"$TARGET"/debian
  rm -rf target/"$TARGET"/generate-rpm
  if [ -f /etc/redhat-release ]; then
    cd crate/server && cargo build --target "$TARGET" --release && cd -
    cargo install --version 0.16.0 cargo-generate-rpm --force
    cd "$ROOT_FOLDER"
    cargo generate-rpm --target "$TARGET" -p crate/server --metadata-overwrite=pkg/rpm/scriptlets.toml
  elif [ -f /etc/lsb-release ]; then
    cargo install --version 2.4.0 cargo-deb --force
    cargo deb --target "$TARGET" -p cosmian_findex_server
  fi
fi

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
cargo build --target $TARGET $RELEASE

if [ "$DEBUG_OR_RELEASE" = "release" ]; then
  INCLUDE_IGNORED="--include-ignored"
fi
export RUST_LOG="fatal,cosmian_cli=error,cosmian_findex_client=debug,cosmian_findex_server=debug"
# shellcheck disable=SC2086
cargo test --workspace --lib --target $TARGET $RELEASE $FEATURES -- --nocapture $SKIP_SERVICES_TESTS $INCLUDE_IGNORED
