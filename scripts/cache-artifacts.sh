#!/bin/bash
# cache-artifacts.sh - Helper script to cache large artifacts for fast development iteration
#
# This script copies pre-downloaded artifacts from ~/stash/ to ~/.foc-localnet/artifacts/
# to avoid re-downloading them during init.

set -e

STASH_DIR="${STASH_DIR:-$HOME/stash}"
FOC_ARTIFACTS="$HOME/.foc-localnet/artifacts"
FOC_VOLUMES="$FOC_ARTIFACTS/docker/volumes"

echo "🚀 Caching artifacts for fast init..."
echo

# Check if stash directory exists
if [ ! -d "$STASH_DIR" ]; then
    echo "❌ Error: Stash directory not found at $STASH_DIR"
    echo "   Set STASH_DIR environment variable if it's in a different location."
    exit 1
fi

# Create artifacts directory
mkdir -p "$FOC_ARTIFACTS"
echo "✅ Created artifacts directory: $FOC_ARTIFACTS"

# Cache Yugabyte tarball
echo
echo "📦 Caching Yugabyte tarball..."
if [ -f "$STASH_DIR/yugabyte/yugabyte-2.25.1.0-b381-linux-x86_64.tar.gz" ]; then
    cp "$STASH_DIR/yugabyte/yugabyte-2.25.1.0-b381-linux-x86_64.tar.gz" "$FOC_ARTIFACTS/"
    echo "✅ Yugabyte cached (~422 MB)"
else
    echo "⚠️  Warning: Yugabyte tarball not found in $STASH_DIR/yugabyte/"
    echo "   This will be downloaded during init (~422 MB)"
fi

# Cache proof parameters (only if volumes directory exists)
echo
echo "📦 Caching Filecoin proof parameters..."
if [ -d "$STASH_DIR/filecoin-proof-parameters" ]; then
    if [ -d "$FOC_VOLUMES" ]; then
        mkdir -p "$FOC_VOLUMES/filecoin-proof-parameters"
        echo "   Copying proof parameters (~2 GB, this may take a minute)..."
        cp -r "$STASH_DIR/filecoin-proof-parameters/"* "$FOC_VOLUMES/filecoin-proof-parameters/"
        echo "✅ Proof parameters cached (~2 GB)"
    else
        echo "⚠️  Skipping proof parameters (volumes directory doesn't exist yet)"
        echo "   Run this script again after 'cargo run init' completes"
        echo "   Or manually copy with:"
        echo "     mkdir -p $FOC_VOLUMES/filecoin-proof-parameters"
        echo "     cp -r $STASH_DIR/filecoin-proof-parameters/* $FOC_VOLUMES/filecoin-proof-parameters/"
    fi
else
    echo "⚠️  Warning: Proof parameters not found in $STASH_DIR/filecoin-proof-parameters/"
    echo "   These will be downloaded during first build (~2 GB)"
fi

echo
echo "✅ Artifact caching complete!"
echo
echo "💡 Quick init command with local repositories:"
echo "   cargo run init \\"
echo "     --curio local:$HOME/code/curio \\"
echo "     --filecoin-services local:$HOME/code/filecoin-services \\"
echo "     --lotus local:$HOME/code/lotus"
echo
