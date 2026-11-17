#!/bin/sh
# Two commands per release.
#
#   scripts/release.sh 0.6.0         Prepare: move CHANGELOG.md "Unreleased" under 0.6.0, set the
#                                    version in package.json, tauri.conf.json, Cargo.toml and
#                                    Cargo.lock, push branch release/0.6.0 and open its pull request.
#   scripts/release.sh --tag 0.6.0   After that pull request is merged: tag main and push the tag,
#                                    which starts the Release workflow (approve it in Actions).
#
# Needs git, gh (logged in), perl and cargo. Run from anywhere inside the repository.
set -eu

REPO_URL="https://github.com/MarcAparisiTFG/portkeeper"
die() { echo "release: $*" >&2; exit 1; }

TAG_ONLY=""
if [ "${1:-}" = "--tag" ]; then TAG_ONLY=1; shift; fi
VER="${1:-}"; VER="${VER#v}"
echo "$VER" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$' || die "usage: $0 [--tag] <major.minor.patch>"

cd "$(git rev-parse --show-toplevel)"
git fetch -q origin --tags

if [ -n "$TAG_ONLY" ]; then
  # Tag exactly what is on GitHub's main, and only once the release is merged there.
  git rev-parse -q --verify "refs/tags/v$VER" >/dev/null && die "tag v$VER already exists"
  MAIN="$(git rev-parse origin/main)"
  git show "$MAIN:package.json" | grep -q "\"version\": \"$VER\"" \
    || die "main is not at $VER yet: merge the 'Release $VER' pull request first"
  git show "$MAIN:CHANGELOG.md" | grep -q "^## \[$VER\]" \
    || die "main's CHANGELOG.md has no $VER section"
  git tag "v$VER" "$MAIN"
  git push origin "v$VER"
  echo
  echo "Tagged v$VER at $(git log --oneline -1 "$MAIN")."
  echo "Approve the build: $REPO_URL/actions/workflows/release.yml (Review deployments -> release)."
  exit 0
fi

[ -z "$(git status --porcelain --untracked-files=no)" ] || die "commit or stash your changes first"
git rev-parse -q --verify "refs/tags/v$VER" >/dev/null && die "v$VER is already released"
grep -q "^## \[$VER\]" CHANGELOG.md && die "CHANGELOG.md already has a $VER section"

# Something must be waiting under Unreleased.
NOTES="$(awk '/^## \[Unreleased\]/{on=1;next} /^## /{on=0} on && NF' CHANGELOG.md)"
[ -n "$NOTES" ] || die "nothing under ## [Unreleased] in CHANGELOG.md"

PREV="$(grep -Eo '^## \[[0-9]+\.[0-9]+\.[0-9]+\]' CHANGELOG.md | head -1 | tr -d '#[] ')"
[ -n "$PREV" ] || die "no previous version found in CHANGELOG.md"
DATE="$(date +%Y-%m-%d)"

git checkout -q -B "release/$VER" origin/main

# CHANGELOG.md: new section under a fresh, empty Unreleased; compare links.
perl -0pi -e "s/^## \[Unreleased\]\n/## [Unreleased]\n\n## [$VER] - $DATE\n/m" CHANGELOG.md
perl -0pi -e "s{^\[Unreleased\]: \S+\n}{[Unreleased]: $REPO_URL/compare/v$VER...HEAD\n[$VER]: $REPO_URL/compare/v$PREV...v$VER\n}m" CHANGELOG.md

# Version in the three files (first occurrence only), then Cargo.lock.
perl -0pi -e "s/\"version\": \"[^\"]+\"/\"version\": \"$VER\"/" package.json src-tauri/tauri.conf.json
perl -0pi -e "s/^version = \"[^\"]+\"/version = \"$VER\"/m" src-tauri/Cargo.toml
(cd src-tauri && cargo update -q -p portkeeper --offline)

# Every file must agree before anything is pushed.
for f in package.json src-tauri/tauri.conf.json; do
  grep -q "\"version\": \"$VER\"" "$f" || die "version not set in $f"
done
grep -q "^version = \"$VER\"" src-tauri/Cargo.toml || die "version not set in Cargo.toml"
grep -A1 '^name = "portkeeper"$' src-tauri/Cargo.lock | grep -q "version = \"$VER\"" || die "Cargo.lock not updated"
BODY_NOTES="$(sh .github/release-notes.sh "$VER")"

git add CHANGELOG.md package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -q -m "Release $VER"
git push -q -u origin "release/$VER"
gh pr create --title "Release $VER" --body "Cuts $VER: CHANGELOG.md \"Unreleased\" moves under $VER ($DATE) and the version is set in package.json, tauri.conf.json, Cargo.toml and Cargo.lock.

## Release notes (what GitHub, the website and the app will show)

$BODY_NOTES

## After merging

\`scripts/release.sh --tag $VER\`, then approve the \`release\` deployment in Actions."

echo
echo "Next: merge that pull request once CI is green, then run: scripts/release.sh --tag $VER"
