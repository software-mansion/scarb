#!/usr/bin/env bash
set -euxo pipefail

mkdir -p target/verify

SCARB_ARCHIVE="$1"
EXPECTED_VERSION="$2"

# Trim leading v character if exists.
EXPECTED_VERSION="${EXPECTED_VERSION//v/}"

if echo "$SCARB_ARCHIVE" | grep -Fq .tar.gz; then
  tar -zxvf "$SCARB_ARCHIVE" --strip-components=1 -C target/verify
else
  7z x -y "$SCARB_ARCHIVE" -otarget
  mv target/"$(basename "$SCARB_ARCHIVE" .zip)"/* target/verify/
fi

SCARB=$(find target/verify/bin -name 'scarb' -o -name 'scarb.exe')

"$SCARB" --version

"$SCARB" --version | grep -Fq "$EXPECTED_VERSION"

"$SCARB" --help

"$SCARB" new ci_testing
"$SCARB" --manifest-path=ci_testing/Scarb.toml build
rm -rf ci_testing
