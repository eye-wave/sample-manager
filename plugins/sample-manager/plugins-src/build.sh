#!/bin/sh
# Builds the example plugins and copies them into the plugins folder of the app.
# Needs the target: rustup target add wasm32-wasip2
set -e
cd "$(dirname "$0")"
cargo build --release --target wasm32-wasip2 -Z build-std=std -Z unstable-options
dest="${1:-${XDG_CONFIG_HOME:-$HOME/.config}/sample-manager/plugins}"
mkdir -p "$dest"
for p in splice; do
  cp "target/wasm32-wasip2/release/$(echo $p | tr - _).wasm" "$dest/$p.wasm"
done
echo "installed in $dest"
