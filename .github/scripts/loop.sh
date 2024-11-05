#!/bin/bash

set -ex

cargo build

# export RUST_LOG="cosmian_findex_cli=trace,cosmian_findex_server=trace,test_findex_server=trace"

echo "Running tests in an infinite loop"
while true; do
  reset
  # cargo test --workspace -- --nocapture
  cargo nextest run --workspace --nocapture
  sleep 1
done
