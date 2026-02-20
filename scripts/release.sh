#!/bin/bash
set -e

# Usage: ./scripts/release.sh <bump> [-y]
# bump: patch | minor | major | alpha | beta | rc | release
# -y: Skip confirmation prompt
#
# Examples:
#   0.0.1-alpha  + patch   → 0.0.2-alpha
#   0.0.1-alpha  + minor   → 0.1.0-alpha
#   0.0.1-alpha  + major   → 1.0.0-alpha
#   0.0.1-alpha  + alpha   → 0.0.1-alpha.2
#   0.0.1-alpha  + beta    → 0.0.1-beta
#   0.0.1-beta   + rc      → 0.0.1-rc
#   0.0.1-rc     + release → 0.0.1

BUMP=$1
AUTO_CONFIRM=$2

if [ -z "$BUMP" ]; then
  echo "Usage: $0 <bump> [-y]"
  echo "bump: patch | minor | major | alpha | beta | rc | release"
  exit 1
fi

# Get current version from Cargo.toml
CURRENT_VERSION=$(grep -m1 '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
echo "Current version: $CURRENT_VERSION"

# Parse version components
if [[ "$CURRENT_VERSION" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)(-([a-zA-Z]+)(\.([0-9]+))?)?$ ]]; then
  MAJOR="${BASH_REMATCH[1]}"
  MINOR="${BASH_REMATCH[2]}"
  PATCH="${BASH_REMATCH[3]}"
  PRERELEASE="${BASH_REMATCH[5]}"
  PRERELEASE_NUM="${BASH_REMATCH[7]}"
else
  echo "Error: Cannot parse version: $CURRENT_VERSION"
  exit 1
fi

# Calculate new version based on bump type
case "$BUMP" in
  patch)
    PATCH=$((PATCH + 1))
    if [ -n "$PRERELEASE" ]; then
      NEW_VERSION="$MAJOR.$MINOR.$PATCH-$PRERELEASE"
    else
      NEW_VERSION="$MAJOR.$MINOR.$PATCH"
    fi
    ;;
  minor)
    MINOR=$((MINOR + 1))
    PATCH=0
    if [ -n "$PRERELEASE" ]; then
      NEW_VERSION="$MAJOR.$MINOR.$PATCH-$PRERELEASE"
    else
      NEW_VERSION="$MAJOR.$MINOR.$PATCH"
    fi
    ;;
  major)
    MAJOR=$((MAJOR + 1))
    MINOR=0
    PATCH=0
    if [ -n "$PRERELEASE" ]; then
      NEW_VERSION="$MAJOR.$MINOR.$PATCH-$PRERELEASE"
    else
      NEW_VERSION="$MAJOR.$MINOR.$PATCH"
    fi
    ;;
  alpha)
    if [ "$PRERELEASE" = "alpha" ]; then
      if [ -n "$PRERELEASE_NUM" ]; then
        PRERELEASE_NUM=$((PRERELEASE_NUM + 1))
      else
        PRERELEASE_NUM=2
      fi
      NEW_VERSION="$MAJOR.$MINOR.$PATCH-alpha.$PRERELEASE_NUM"
    else
      NEW_VERSION="$MAJOR.$MINOR.$PATCH-alpha"
    fi
    ;;
  beta)
    NEW_VERSION="$MAJOR.$MINOR.$PATCH-beta"
    ;;
  rc)
    NEW_VERSION="$MAJOR.$MINOR.$PATCH-rc"
    ;;
  release)
    NEW_VERSION="$MAJOR.$MINOR.$PATCH"
    ;;
  *)
    echo "Error: Unknown bump type: $BUMP"
    echo "Valid options: patch | minor | major | alpha | beta | rc | release"
    exit 1
    ;;
esac

TAG="v$NEW_VERSION"

echo "New version: $NEW_VERSION (tag: $TAG)"
echo ""

# Confirm (skip with -y flag)
if [ "$AUTO_CONFIRM" != "-y" ]; then
  read -p "Proceed with release? [y/N] " -n 1 -r
  echo
  if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 1
  fi
fi

# Check for uncommitted changes
if [ -n "$(git status --porcelain)" ]; then
  echo "Error: There are uncommitted changes. Please commit or stash them first."
  exit 1
fi

# Check if tag already exists
if git rev-parse "$TAG" >/dev/null 2>&1; then
  echo "Error: Tag $TAG already exists."
  exit 1
fi

# Update Cargo.toml (workspace version and dependencies)
echo "Updating Cargo.toml..."
sed -i.bak 's/^version = ".*"/version = "'"$NEW_VERSION"'"/' Cargo.toml
# Update internal crate versions in workspace.dependencies
sed -i.bak 's/version = "'"$CURRENT_VERSION"'"/version = "'"$NEW_VERSION"'"/g' Cargo.toml
rm -f Cargo.toml.bak

# Update npm package versions
echo "Updating npm packages..."
for pkg in npm/*/; do
  pkg=${pkg%/}  # remove trailing slash
  if [ -f "$pkg/package.json" ]; then
    node -e "
      const fs = require('fs');
      const pkg = JSON.parse(fs.readFileSync('$pkg/package.json', 'utf8'));
      pkg.version = '$NEW_VERSION';
      // Update optionalDependencies versions for native packages (NAPI bindings only)
      // NOTE: @vizejs/cli-* are NOT bumped here because they are published by the
      // Release workflow. Bumping them here would break the lockfile since the new
      // version doesn't exist on npm yet. The release workflow injects the correct
      // version at publish time.
      if (pkg.optionalDependencies) {
        for (const dep of Object.keys(pkg.optionalDependencies)) {
          if (dep.startsWith('@vizejs/native-') || dep.startsWith('@vizejs/fresco-native-')) {
            pkg.optionalDependencies[dep] = '$NEW_VERSION';
          }
        }
      }
      fs.writeFileSync('$pkg/package.json', JSON.stringify(pkg, null, 2) + '\n');
    "
    echo "  Updated $pkg/package.json"
  fi
done

# Update version references in READMEs
echo "Updating READMEs..."
find npm -name 'README.md' -exec sed -i.bak "s/$CURRENT_VERSION/$NEW_VERSION/g" {} \;
find npm -name 'README.md.bak' -delete
sed -i.bak "s/$CURRENT_VERSION/$NEW_VERSION/g" README.md
rm -f README.md.bak

# Commit changes
echo "Committing changes..."
git add Cargo.toml npm/*/package.json README.md
# Add READMEs that are tracked (some may be gitignored)
git add npm/*/README.md 2>/dev/null || true
git commit -m "chore: release v$NEW_VERSION"

# Create tag
echo "Creating tag $TAG..."
git tag -a "$TAG" -m "Release $NEW_VERSION"

# Push to remote
echo "Pushing to remote..."
git push origin main
git push origin "$TAG"

echo ""
echo "Release $NEW_VERSION complete!"
echo "GitHub Actions will now publish to npm and crates.io."
echo "Check: https://github.com/ubugeeei/vize/actions"
