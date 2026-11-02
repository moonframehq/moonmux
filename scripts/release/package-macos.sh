#!/usr/bin/env bash
# Packages the macOS release in OUT_DIR from one executable per
# architecture: a universal Moonmux.app (the app's own `bundle` command:
# the binary, its app folder and Moonframe's toolbox with its Wasm providers
# prebuilt, signed ad hoc), Moonmux.app.tar for the signing job, and the
# disk image users download, moonmux-VERSION-universal-apple-darwin.dmg.
# Developer ID signing and notarization happen later, in
# scripts/release/sign-macos.sh.
set -euo pipefail
[[ $# -ge 2 ]] || { echo "usage: package-macos.sh OUT_DIR BINARY..." >&2; exit 2; }
[[ $(uname -s) == Darwin ]] || { echo "macOS required" >&2; exit 1; }
out=$1
shift
version=$(scripts/release/version.sh)
mkdir -p "$out"
out=$(cd "$out" && pwd)

work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT
lipo -create "$@" -output "$work/moonmux"
lipo -info "$work/moonmux"

# Outside the checkout's target/, the binary runs the app compiled into it,
# which is what it bundles. It runs from here so cargo builds the Wasm
# providers with the pinned toolchain.
"$work/moonmux" bundle "$work/bundle" >/dev/null
app="$work/bundle/Moonmux.app"
codesign --verify --strict "$app"
# The bundled executable must find the app inside the bundle and load every
# module of it, headlessly, on each architecture this Mac can run.
for arch in $(lipo -archs "$app/Contents/MacOS/moonmux"); do
    if arch "-$arch" /usr/bin/true 2>/dev/null; then
        arch "-$arch" "$app/Contents/MacOS/moonmux" --check --no-user
    else
        echo "skipping the $arch check: this Mac cannot run it" >&2
    fi
done

tar -C "$work/bundle" -cf "$out/Moonmux.app.tar" Moonmux.app
scripts/release/make-dmg.sh "$app" "$out/moonmux-$version-universal-apple-darwin.dmg"
