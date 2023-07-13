#!/usr/bin/env bash
set -euxo pipefail

TARGET="$1"
STAGING="$2"

rm -rf "$STAGING"
mkdir -p "$STAGING"

bin_ext=""
[[ "$TARGET" == *-windows-* ]] && bin_ext=".exe"

mkdir -p \
  "$STAGING/bin/" \
  "$STAGING/doc/"

for crate in $(cargo xtask list-binaries); do
  cp "target/$TARGET/release/${crate}${bin_ext}" "$STAGING/bin/"
done

cp -r README.md SECURITY.md LICENSE "$STAGING/doc/"

if [[ "$TARGET" == *-windows-* ]]; then
  7z a "${STAGING}.zip" "$STAGING"
else
  tar czvf "${STAGING}.tar.gz" "$STAGING"
fi
