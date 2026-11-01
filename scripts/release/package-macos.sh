#!/usr/bin/env bash
# Packages the macOS release in OUT_DIR from one executable per
# architecture: a universal Herdr.app (the app's own `bundle` command:
# the binary, its app folder and Moonframe's toolbox with its Wasm providers
# prebuilt, signed ad hoc), Herdr.app.tar for the signing job, and the
# disk image users download, herdr-moonframe-VERSION-universal-apple-darwin.dmg.
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
lipo -create "$@" -output "$work/herdr-moonframe"
lipo -info "$work/herdr-moonframe"

# Outside the checkout's target/, the binary runs the app compiled into it,
# which is what it bundles. It runs from here so cargo builds the Wasm
# providers with the pinned toolchain.
"$work/herdr-moonframe" bundle "$work/bundle" >/dev/null
app="$work/bundle/Herdr.app"
codesign --verify --strict "$app"
# The bundled executable must find the app inside the bundle and load every
# module of it, headlessly, on each architecture this Mac can run.
for arch in $(lipo -archs "$app/Contents/MacOS/herdr-moonframe"); do
    if arch "-$arch" /usr/bin/true 2>/dev/null; then
        arch "-$arch" "$app/Contents/MacOS/herdr-moonframe" --check --no-user
    else
        echo "skipping the $arch check: this Mac cannot run it" >&2
    fi
done

tar -C "$work/bundle" -cf "$out/Herdr.app.tar" Herdr.app
scripts/release/make-dmg.sh "$app" "$out/herdr-moonframe-$version-universal-apple-darwin.dmg"
