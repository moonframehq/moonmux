#!/usr/bin/env bash
# Install what `just ci`, `just audit-workflows`, `just audit-deps`,
# `just check-commits` and `just release-check` need on a developer machine,
# so a local run can match CI. macOS uses Homebrew; Linux uses apt (Ubuntu
# 24.04, like the CI runners) and Cargo for the rest. Safe to rerun.
set -euo pipefail

fail() { printf '%s\n' "$*" >&2; exit 1; }
have() { command -v "$1" >/dev/null 2>&1; }

root=$(git rev-parse --show-toplevel)
have rustup || fail 'rustup is required: https://rustup.rs'
# The pinned toolchain from rust-toolchain.toml, as CI installs it.
(cd "$root" && rustup toolchain install)

case "$(uname -s)" in
    Darwin)
        have brew || fail 'Homebrew is required: https://brew.sh'
        packages=()
        for tool in actionlint zizmor cargo-deny git-cliff taplo jq; do
            have "$tool" || packages+=("$tool")
        done
        if ((${#packages[@]})); then
            brew install "${packages[@]}"
        fi
        ;;
    Linux)
        bash "$root/scripts/install-linux-deps.sh"
        sudo apt-get install --no-install-recommends -y jq python3 pipx
        have zizmor || pipx install zizmor
        for crate in cargo-deny git-cliff taplo-cli; do
            binary=${crate%-cli}
            have "$binary" || cargo install --locked "$crate"
        done
        have actionlint || go install github.com/rhysd/actionlint/cmd/actionlint@latest \
            || fail 'actionlint: install Go, or see https://github.com/rhysd/actionlint'
        ;;
    *) fail "Unsupported platform: $(uname -s)" ;;
esac
# The pinned luau-lsp `just types` uses, into target/tools.
(cd "$root" && scripts/setup-luau-lsp.sh >/dev/null)
echo "CI tools are installed."
