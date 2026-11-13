default:
    @just --list

# Open Moonmux: spaces of terminals kept by herdr-core or tmux, and the agents in them.
run *args:
    cargo run --release -- {{args}}

# The same app drawn by Rune, Moonframe's experimental renderer, instead of
# GPUI; built in target/rune, so it never rebuilds over `just run`.
run-rune *args:
    cargo run --release --target-dir target/rune --no-default-features --features rune,herdr-core,tmux -- {{args}}

# Load the app headlessly: every mod, component and provider, without a window.
check:
    cargo run --locked -- --check

# The same headless load, on Rune.
check-rune:
    cargo run --locked --target-dir target/rune --no-default-features --features rune,herdr-core,tmux -- --check

# Type-check the app's Luau against types/moonframe.d.luau in strict mode.
types:
    #!/usr/bin/env bash
    set -euo pipefail
    lsp=$(scripts/setup-luau-lsp.sh)
    files=$(find app -name '*.luau' -not -name '*.d.luau' | sort)
    output=$("$lsp" analyze --platform=standard --definitions=@moonframe=types/moonframe.d.luau $files 2>&1 | grep -v '^\[INFO\]' || true)
    if [ -n "$output" ]; then
        echo "$output"
        exit 1
    fi
    echo "ok: types in app"

# Format Rust and TOML.
format:
    cargo fmt --all
    taplo fmt

# Check Rust and TOML formatting without writing.
format-check:
    cargo fmt --all -- --check
    taplo fmt --check

lint:
    cargo clippy --locked --all-targets -- -D warnings

test:
    cargo test --locked

# Clippy and tests with each terminal backend alone, and on Rune, as CI
# runs them.
features:
    cargo clippy --locked --all-targets --no-default-features --features gpui,herdr-core -- -D warnings
    cargo test --locked --no-default-features --features gpui,herdr-core
    cargo clippy --locked --all-targets --no-default-features --features gpui,tmux -- -D warnings
    cargo test --locked --no-default-features --features gpui,tmux
    cargo clippy --locked --all-targets --target-dir target/rune --no-default-features --features rune,herdr-core,tmux -- -D warnings
    cargo test --locked --target-dir target/rune --no-default-features --features rune,herdr-core,tmux

check-file-size:
    bash scripts/check-file-size.sh

check-sorted:
    python3 scripts/test_check_sorted.py
    python3 scripts/check-sorted.py

# Sort every keep-sorted region in place.
sort:
    python3 scripts/check-sorted.py --fix

# What CI runs.
ci: format-check check-file-size check-sorted lint test features check types

# Install the tools local CI-equivalent runs need, on macOS or Ubuntu.
setup-ci:
    bash scripts/setup-ci.sh

# Audit the GitHub workflows for injection, over-broad permissions, and
# unpinned actions.
audit-workflows:
    zizmor .github/
    actionlint

# License, advisory, ban and source checks over the dependency graph.
audit-deps:
    cargo deny check

# Check commit subjects the way CI does; the changelog is generated from them.
check-commits base="origin/main":
    python3 scripts/release/check-commit-messages.py range "{{base}}..HEAD"

# Install the hooks, and the merge settings that cut repeated conflict work,
# for this checkout and every worktree of it.
hooks:
    git config core.hooksPath .githooks
    git config rerere.enabled true
    git config rerere.autoupdate true
    git config merge.cargo-lock.name "Cargo.lock: keep one side, then rebuild with Cargo"
    git config merge.cargo-lock.driver "bash scripts/git/merge-cargo-lock.sh %O %A %B"
    @echo "Hooks, rerere, and the Cargo.lock merge driver are configured."

# Move Cargo.lock to the latest commit on Moonframe's main, then run `just ci`.
moonframe:
    cargo update -p moonframe
    @git diff --stat Cargo.lock

# What changed since the last release tag, straight from git history.
changelog-unreleased:
    git-cliff --config cliff.toml --unreleased

# Exactly what a release would publish: CHANGELOG.md and RELEASE_NOTES.md in OUTPUT.
changelog-release output="target/changelog":
    mkdir -p {{output}}
    rm -f {{output}}/CHANGELOG.md {{output}}/RELEASE_NOTES.md
    MACOS_SIGNED=${MACOS_SIGNED:-true} WINDOWS_SIGNED=${WINDOWS_SIGNED:-true} \
        scripts/release/generate-changelog.sh "$(scripts/release/version.sh)" {{output}}

# Build this Mac's release packages, unsigned, as .github/workflows/package.yml
# does: Moonmux.app and its disk image in OUT.
package-macos out="target/dist":
    cargo build --locked --release
    scripts/release/package-macos.sh {{out}} target/release/moonmux

# Build a standalone app for this machine: target/bundle/Moonmux.app on macOS.
bundle:
    cargo run --release -- bundle

# Render the app icon (assets/icon.png, app/icon.png) from assets/icon.svg.
icon:
    rsvg-convert -w 1024 -h 1024 assets/icon.svg -o assets/icon.png
    cp assets/icon.png app/icon.png

# Static checks of the release scripts and workflows; publishes nothing.
release-check:
    for script in scripts/*.sh scripts/release/*.sh scripts/git/*.sh; do bash -n "$script" || exit; done
    actionlint
    zizmor .github/

# Publish main's HEAD as v<version in Cargo.toml> (bump it first) through the
# Release workflow, which waits for CI on that commit; see docs/releasing.md.
release:
    #!/usr/bin/env bash
    set -euo pipefail
    git fetch --quiet origin main
    sha=$(git rev-parse origin/main)
    version=$(scripts/release/version.sh)
    test "$(git rev-parse HEAD)" = "$sha" || { echo "check out origin/main ($sha) first" >&2; exit 1; }
    test -z "$(git status --porcelain)" || { echo "the working tree is not clean" >&2; exit 1; }
    echo "Releasing v$version from $sha"
    gh workflow run release.yml --ref main -f expected_sha="$sha"
