#!/bin/bash

# Script to bump version in Cargo.toml and package.json
# Usage: ./bump_version.sh [major|minor|patch]

# Check if parameter is provided
if [ $# -eq 0 ]; then
    echo "Error: Please provide version bump type (major, minor, or patch)"
    echo "Usage: $0 [major|minor|patch]"
    exit 1
fi

# Validate parameter
BUMP_TYPE=$1
if [[ ! "$BUMP_TYPE" =~ ^(major|minor|patch)$ ]]; then
    echo "Error: Invalid bump type. Must be 'major', 'minor', or 'patch'"
    exit 1
fi

# Get the directory where the script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"

# File paths
CARGO_TOML="$PROJECT_ROOT/Cargo.toml"
PACKAGE_JSON="$PROJECT_ROOT/package.json"

# Check if files exist
if [ ! -f "$CARGO_TOML" ]; then
    echo "Error: Cargo.toml not found at $CARGO_TOML"
    exit 1
fi

if [ ! -f "$PACKAGE_JSON" ]; then
    echo "Error: package.json not found at $PACKAGE_JSON"
    exit 1
fi

# Extract current version from Cargo.toml
CURRENT_VERSION=$(grep -E '^version = ".*"' "$CARGO_TOML" | sed -E 's/version = "(.*)"/\1/')

if [ -z "$CURRENT_VERSION" ]; then
    echo "Error: Could not find version in Cargo.toml"
    exit 1
fi

echo "Current version: $CURRENT_VERSION"

# Split version into components
IFS='.' read -r -a VERSION_PARTS <<< "$CURRENT_VERSION"
MAJOR=${VERSION_PARTS[0]}
MINOR=${VERSION_PARTS[1]}
PATCH=${VERSION_PARTS[2]}

# Validate version components
if [ -z "$MAJOR" ] || [ -z "$MINOR" ] || [ -z "$PATCH" ]; then
    echo "Error: Invalid version format. Expected format: X.Y.Z"
    exit 1
fi

# Bump the appropriate version component
case $BUMP_TYPE in
    major)
        MAJOR=$((MAJOR + 1))
        MINOR=0
        PATCH=0
        ;;
    minor)
        MINOR=$((MINOR + 1))
        PATCH=0
        ;;
    patch)
        PATCH=$((PATCH + 1))
        ;;
esac

# Construct new version
NEW_VERSION="$MAJOR.$MINOR.$PATCH"
echo "New version: $NEW_VERSION"

# Update Cargo.toml
echo "Updating Cargo.toml..."
sed -i -E "s/^version = \".*\"/version = \"$NEW_VERSION\"/" "$CARGO_TOML"

# Update package.json
echo "Updating package.json..."
sed -i -E "s/\"version\": \".*\"/\"version\": \"$NEW_VERSION\"/" "$PACKAGE_JSON"

echo "Version bump complete!"
echo "Updated to version $NEW_VERSION in both Cargo.toml and package.json"

# Create git tag
echo "Creating git tag v$NEW_VERSION..."
git tag -a "v$NEW_VERSION" -m "version v$NEW_VERSION"

if [ $? -eq 0 ]; then
    echo "Git tag v$NEW_VERSION created successfully"
    
    # Push changes and tags
    echo "Pushing changes to remote..."
    git push && git push --tags
    
    if [ $? -eq 0 ]; then
        echo "Successfully pushed changes and tags to remote"
    else
        echo "Error: Failed to push changes or tags to remote"
        exit 1
    fi
else
    echo "Error: Failed to create git tag"
    exit 1
fi
