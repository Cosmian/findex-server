#!/bin/sh

set -ex

OLD_VERSION="$1"
NEW_VERSION="$2"
# Use SED_BINARY from environment if set, otherwise default to 'sed'
# On MacOs - install gnu-sed with brew
SED_BINARY=${SED_BINARY:-sed}

${SED_BINARY} -i "s/$OLD_VERSION/$NEW_VERSION/g" Cargo.toml

# Other files
${SED_BINARY} -i "s/$OLD_VERSION/$NEW_VERSION/g" Dockerfile
${SED_BINARY} -i "s/$OLD_VERSION/$NEW_VERSION/g" documentation/docs/quick_start.md
${SED_BINARY} -i "s/$OLD_VERSION/$NEW_VERSION/g" README.md

cargo build
git cliff -u -p CHANGELOG.md -t "$NEW_VERSION"
