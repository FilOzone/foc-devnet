#!/usr/bin/env bash
# Run from the selected filecoin-services service_contracts directory.
set -euo pipefail

# Let Make expand the checkout's definitions, including any included files.
evm_config="$(make --silent --no-print-directory -f Makefile -f - foc-devnet-evm-config <<'MAKEFILE'
.PHONY: foc-devnet-evm-config
foc-devnet-evm-config:
	@printf '%s|%s\n' "$(EVM_REPO)" "$(EVM_REVISION)"
MAKEFILE
)"
IFS='|' read -r evm_repo evm_revision <<< "$evm_config"

# Releases predating ERC-8167 do not declare an assembler dependency.
if [[ -z "$evm_repo" && -z "$evm_revision" ]]; then
    exit 0
fi
: "${evm_repo:?filecoin-services must define EVM_REPO when EVM_REVISION is set}"
: "${evm_revision:?filecoin-services must define EVM_REVISION when EVM_REPO is set}"

echo "Installing evm revision $evm_revision from $evm_repo"
evm_source="$(mktemp -d)"
trap 'rm -rf "$evm_source"' EXIT

# Match our repository setup flow; Ubuntu 24.04's Git lacks clone --revision.
git clone -- "$evm_repo" "$evm_source"
git -C "$evm_source" checkout --detach "$evm_revision"
git -C "$evm_source" submodule update --init --recursive
make -C "$evm_source" bin/evm
mkdir -p "$HOME/.local/bin"
install "$evm_source/bin/evm" "$HOME/.local/bin/evm"
