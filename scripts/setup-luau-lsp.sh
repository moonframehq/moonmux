#!/usr/bin/env bash
# Download the pinned luau-lsp, whose `analyze` type-checks the app against
# types/moonframe.d.luau, into target/tools: the release's macOS or Linux
# build, checked against its pinned SHA-256.
set -euo pipefail
version="1.70.1"
case "$(uname -s)-$(uname -m)" in
Darwin-*)
    asset=luau-lsp-macos.zip
    sha256="7d3936e8dec6dc77547abd061d2e950da392562a5f94858b880f23dba50d84cd"
    ;;
Linux-x86_64)
    asset=luau-lsp-linux-x86_64.zip
    sha256="1a2ea1ae4f98f8946cefd970a4b54853e11ab4775e6932faa7f60cf920346567"
    ;;
Linux-aarch64)
    asset=luau-lsp-linux-arm64.zip
    sha256="6062345b95124ca937b8c324b74bc3fddb1d6919ba133c58f8b17945ce1acbc2"
    ;;
*)
    echo "no pinned luau-lsp for $(uname -s) $(uname -m); see https://github.com/JohnnyMorganz/luau-lsp/releases" >&2
    exit 1
    ;;
esac
dir="target/tools/luau-lsp-$version"
if [ -x "$dir/luau-lsp" ]; then
    echo "$dir/luau-lsp"
    exit 0
fi
download="target/tools/download"
mkdir -p "$download"
curl --fail --location --silent --show-error \
    --output "$download/$asset" \
    "https://github.com/JohnnyMorganz/luau-lsp/releases/download/$version/$asset"
echo "$sha256  $download/$asset" | shasum -a 256 --check --status \
    || { echo "$asset does not match its pinned checksum" >&2; exit 1; }
unzip -o -q "$download/$asset" -d "$dir"
chmod +x "$dir/luau-lsp"
echo "$dir/luau-lsp"
