#!/usr/bin/env bash
# Writes CHANGELOG.md and RELEASE_NOTES.md for v<VERSION> into OUTPUT_DIR,
# from git history through git-cliff (cliff.toml). Neither is tracked in git:
# the release workflow passes RELEASE_NOTES.md to `gh release create`.
set -euo pipefail
[[ $# == 2 ]] || { echo "usage: generate-changelog.sh VERSION OUTPUT_DIR" >&2; exit 2; }
version=$1
[[ $version =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]] || { echo "not a version: $version" >&2; exit 1; }
[[ -d $2 ]] || { echo "existing output directory required: $2" >&2; exit 1; }
command -v git-cliff >/dev/null || { echo "git-cliff is required: cargo install --locked git-cliff" >&2; exit 1; }
out=$(cd -- "$2" && pwd)
for file in CHANGELOG.md RELEASE_NOTES.md; do
    [[ ! -e $out/$file ]] || { echo "refusing to overwrite $out/$file" >&2; exit 1; }
done

# Signed builds are stated only once release.yml's validate job finds their
# signing set up. No default, so a preview never promises a signature.
case ${MACOS_SIGNED:-} in
    true) macos='signed and notarized universal macOS disk image' ;;
    false) macos='universal macOS disk image, signed ad hoc (allow it in System Settings → Privacy & Security on first launch)' ;;
    *) echo "MACOS_SIGNED must be true or false" >&2; exit 1 ;;
esac
case ${WINDOWS_SIGNED:-} in
    true) windows='an Authenticode-signed Windows x86_64 zip' ;;
    false) windows='an unsigned Windows x86_64 zip' ;;
    *) echo "WINDOWS_SIGNED must be true or false" >&2; exit 1 ;;
esac

# The tag is created only when the release publishes, so the commits being cut
# are still "unreleased" here and `--tag` names the version they belong to.
cd -- "$(git rev-parse --show-toplevel)"
tag=v$version
source_sha=$(git rev-parse HEAD)
git-cliff --config cliff.toml --tag "$tag" --output "$out/CHANGELOG.md"
cat > "$out/RELEASE_NOTES.md" <<NOTES
A $macos; x86_64 and ARM64 Linux tarballs; and $windows. Terminals run in herdr-core (\`hc\`), which must be installed. Source: $source_sha.

NOTES
git-cliff --config cliff.toml --tag "$tag" --unreleased --strip header >> "$out/RELEASE_NOTES.md"
[[ -s $out/CHANGELOG.md && -s $out/RELEASE_NOTES.md ]] || { echo "the generated changelog is empty" >&2; exit 1; }
echo "$out/RELEASE_NOTES.md"
