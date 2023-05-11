#!/usr/bin/env bash
set -euxo pipefail

SCARB_ARCHIVE="$1"
EXPECTED_VERSION="$2"

# Trim leading v character if exists.
EXPECTED_VERSION="${EXPECTED_VERSION//v/}"

INSTALL_DIR=$(mktemp -d)
mkdir -p "$INSTALL_DIR"

if echo "$SCARB_ARCHIVE" | grep -Fq .tar.gz; then
  tar -zxvf "$SCARB_ARCHIVE" --strip-components=1 -C "$INSTALL_DIR"
else
  7z x -y "$SCARB_ARCHIVE" -otarget
  mv target/"$(basename "$SCARB_ARCHIVE" .zip)"/* "$INSTALL_DIR"
fi

SCARB=$(find "${INSTALL_DIR}/bin" -name 'scarb' -o -name 'scarb.exe')

"$SCARB" --version

"$SCARB" -V | grep -Fq "$EXPECTED_VERSION"

"$SCARB" --help

TEST_DIR=$(mktemp -d)
mkdir -p "$TEST_DIR"

"$SCARB" new "${TEST_DIR}/ci_testing"
"$SCARB" --manifest-path="${TEST_DIR}/ci_testing/Scarb.toml" build
