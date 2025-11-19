#!/bin/bash

# Usage: bash .github/scripts/release.sh 0.4.0 0.4.1

set -ex

OLD_VERSION="$1"
NEW_VERSION="$2"

# Use SED_BINARY from environment if set, otherwise default to 'sed'
# On MacOs - install gnu-sed with brew
SED_BINARY=${SED_BINARY:-sed}

SED() {
  args=$1
  file=$2
  if [[ "$OSTYPE" == "darwin"* ]]; then
    # echo "Not Linux"
    $SED_BINARY -i '' "${args}" "$file"
  else
    # echo "Linux"
    $SED_BINARY -i "${args}" "$file"
  fi
}

# Bump versions in all Cargo.toml
SED "s/$OLD_VERSION/$NEW_VERSION/g" Cargo.toml
find crate -name "Cargo.toml" -exec dirname {} \; | while read -r dir; do
  "SED" "s/$OLD_VERSION/$NEW_VERSION/g" "$dir/Cargo.toml"
done

# Other files
SED "s/$OLD_VERSION/$NEW_VERSION/g" Dockerfile
SED "s/$OLD_VERSION/$NEW_VERSION/g" documentation/docs/quick_start.md
SED "s/$OLD_VERSION/$NEW_VERSION/g" README.md

cargo build
git cliff -w "$(pwd)" -u -p CHANGELOG.md -t "$NEW_VERSION"
