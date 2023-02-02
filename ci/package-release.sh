#!/usr/bin/env bash
set -euxo pipefail

TARGET="$1"
STAGING="$2"

mkdir -p "$STAGING"

bin_ext=""
[[ "$TARGET" == *-windows-* ]] && bin_ext=".exe"

mkdir -p \
  "$STAGING/bin/" \
  "$STAGING/share/doc/scarb/"

for crate in "scarb"; do
  cp "target/$TARGET/release/${crate}${bin_ext}" "$STAGING/bin/"
done

cp -r README.md LICENSE "$STAGING/share/doc/scarb/"

if [[ "$TARGET" == *-windows-* ]]; then
  7z a "${STAGING}.zip" "$STAGING"
else
  tar czvf "${STAGING}.tar.gz" "$STAGING"
fi
