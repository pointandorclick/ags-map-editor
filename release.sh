#!/usr/bin/env bash
set -euo pipefail

if [ $# -ne 1 ]; then
  echo "Usage: ./release.sh <version>"
  echo "Example: ./release.sh 0.2.0"
  exit 1
fi

VERSION="$1"
TAG="v${VERSION}"
CONF="src-tauri/tauri.conf.json"
CARGO="src-tauri/Cargo.toml"
PKG="package.json"

# Validate version format (semver)
if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "Error: Version must be semver (e.g. 0.2.0)"
  exit 1
fi

# Check we're on main
BRANCH=$(git rev-parse --abbrev-ref HEAD)
if [ "$BRANCH" != "main" ]; then
  echo "Error: Must be on main branch (currently on $BRANCH)"
  exit 1
fi

# Check for clean working tree
if ! git diff --quiet || ! git diff --cached --quiet; then
  echo "Error: Working tree is not clean. Commit or stash changes first."
  exit 1
fi

# Check tag doesn't already exist
if git rev-parse "$TAG" >/dev/null 2>&1; then
  echo "Error: Tag $TAG already exists"
  exit 1
fi

# Update tauri.conf.json
sed -i '' "s/\"version\": \".*\"/\"version\": \"${VERSION}\"/" "$CONF"

# Update Cargo.toml (only the first version = line, which is the package version)
sed -i '' "0,/^version = \".*\"/s//version = \"${VERSION}\"/" "$CARGO"

# Update package.json
sed -i '' "s/\"version\": \".*\"/\"version\": \"${VERSION}\"/" "$PKG"

echo "Updated $CONF, $CARGO, and $PKG to $VERSION"

# Commit, tag, push
git add "$CONF" "$CARGO" "$PKG"
git commit -m "Bump version to ${VERSION}"
git tag "$TAG"
git push origin main --tags

echo ""
echo "Done! Tag $TAG pushed."
echo "Monitor the build: https://github.com/pointandorclick/ags-map-editor/actions"
echo "Once complete, publish the draft release: https://github.com/pointandorclick/ags-map-editor/releases"
