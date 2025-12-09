#!/bin/bash
# Patch Lotus buildconstants to make timing parameters settable via linker flags

LOTUS_DIR="${1:-$HOME/code/lotus}"
PARAMS_FILE="$LOTUS_DIR/build/buildconstants/params_localnet.go"

if [ ! -f "$PARAMS_FILE" ]; then
    echo "Error: $PARAMS_FILE not found"
    exit 1
fi

# Check if already patched
if grep -q "blockDelaySecsStr" "$PARAMS_FILE"; then
    echo "Already patched"
    exit 0
fi

echo "Patching $PARAMS_FILE to support linker flags..."

# Create a backup
cp "$PARAMS_FILE" "$PARAMS_FILE.backup"

# Use sed to replace the variable declarations with linker-flag-compatible versions
cat > "$PARAMS_FILE" << 'EOF'
//go:build localnet
// +build localnet

package buildconstants

import (
	_ "embed"
	"strconv"

	"github.com/ipfs/go-cid"

	"github.com/filecoin-project/go-address"
	"github.com/filecoin-project/go-state-types/abi"
	"github.com/filecoin-project/go-state-types/network"
	builtin2 "github.com/filecoin-project/specs-actors/v2/actors/builtin"

	"github.com/filecoin-project/lotus/chain/actors/builtin"
)

var NetworkBundle = "localnet"
var ActorDebugging = true

const BootstrappersFile = ""
const GenesisFile = ""

const GenesisNetworkVersion = network.Version0

var DrandSchedule = map[abi.ChainEpoch]DrandEnum{
	0: DrandQuicknet,
}

var UpgradeBreezeHeight = abi.ChainEpoch(-1)

const BreezeGasTampingDuration = 0

var UpgradeSmokeHeight = abi.ChainEpoch(-1)
var UpgradeIgnitionHeight = abi.ChainEpoch(-2)
var UpgradeRefuelHeight = abi.ChainEpoch(-3)
var UpgradeTapeHeight = abi.ChainEpoch(-4)
var UpgradeAssemblyHeight = abi.ChainEpoch(-5)
var UpgradeLiftoffHeight = abi.ChainEpoch(-6)
var UpgradeKumquatHeight = abi.ChainEpoch(-7)
var UpgradeCalicoHeight = abi.ChainEpoch(-9)
var UpgradePersianHeight = abi.ChainEpoch(-10)
var UpgradeOrangeHeight = abi.ChainEpoch(-11)
var UpgradeClausHeight = abi.ChainEpoch(-12)
var UpgradeTrustHeight = abi.ChainEpoch(-13)
var UpgradeNorwegianHeight = abi.ChainEpoch(-14)
var UpgradeTurboHeight = abi.ChainEpoch(-15)
var UpgradeHyperdriveHeight = abi.ChainEpoch(-16)
var UpgradeChocolateHeight = abi.ChainEpoch(-17)
var UpgradeOhSnapHeight = abi.ChainEpoch(-18)
var UpgradeSkyrHeight = abi.ChainEpoch(-19)
var UpgradeSharkHeight = abi.ChainEpoch(-20)
var UpgradeHyggeHeight = abi.ChainEpoch(-21)
var UpgradeLightningHeight = abi.ChainEpoch(-22)
var UpgradeThunderHeight = abi.ChainEpoch(-23)
var UpgradeWatermelonHeight = abi.ChainEpoch(-24)
var UpgradeDragonHeight = abi.ChainEpoch(-25)
var UpgradePhoenixHeight = abi.ChainEpoch(-26)
var UpgradeWaffleHeight = abi.ChainEpoch(-27)
var UpgradeTeepInitialFilReserved = InitialFilReserved

var UpgradeTuktukHeight = abi.ChainEpoch(-31)

var UpgradeTuktukPowerRampDurationEpochs = uint64(builtin.EpochsInDay * 3)

const UpgradeTeepHeight abi.ChainEpoch = -32

var UpgradeTockHeight abi.ChainEpoch = UpgradeTeepHeight + builtin.EpochsInDay*7

const UpgradeTockFixHeight abi.ChainEpoch = -33

const UpgradeGoldenWeekHeight abi.ChainEpoch = -34

// This fix upgrade only ran on calibrationnet
var UpgradeWatermelonFixHeight = abi.ChainEpoch(-28)

// This fix upgrade only ran on calibrationnet
var UpgradeWatermelonFix2Height = abi.ChainEpoch(-29)

// This fix upgrade only ran on calibrationnet
var UpgradeCalibrationDragonFixHeight = abi.ChainEpoch(-30)

var ConsensusMinerMinPower = abi.NewStoragePower(2048)
var PreCommitChallengeDelay = abi.ChainEpoch(10)

// Make timing parameters configurable via linker flags
// These are string variables that get parsed to uint64 at init time
var blockDelaySecsStr = "30"
var propagationDelaySecsStr = "6"
var equivocationDelaySecsStr = "2"

var BlockDelaySecs uint64
var PropagationDelaySecs uint64
var EquivocationDelaySecs uint64

func init() {
	// Parse string values set by linker flags
	var err error
	BlockDelaySecs, err = strconv.ParseUint(blockDelaySecsStr, 10, 64)
	if err != nil {
		BlockDelaySecs = uint64(builtin2.EpochDurationSeconds)
	}
	PropagationDelaySecs, err = strconv.ParseUint(propagationDelaySecsStr, 10, 64)
	if err != nil {
		PropagationDelaySecs = 6
	}
	EquivocationDelaySecs, err = strconv.ParseUint(equivocationDelaySecsStr, 10, 64)
	if err != nil {
		EquivocationDelaySecs = 2
	}
}

const BootstrapPeerThreshold = 1

// ChainId defines the chain ID used in the Ethereum JSON-RPC endpoint.
// As per https://github.com/ethereum-lists/chains
const Eip155ChainId = 1414

var WhitelistedBlock = cid.Undef

const F3Enabled = false

var F3ManifestBytes []byte

func init() {
	BuildType |= BuildLocalnet
	SetAddressNetwork(address.Testnet)
	Devnet = true
}
EOF

echo "Patch complete! Backup saved to $PARAMS_FILE.backup"
echo "Now the timing parameters can be set via linker flags:"
echo "  -X github.com/filecoin-project/lotus/build/buildconstants.blockDelaySecsStr=2"
echo "  -X github.com/filecoin-project/lotus/build/buildconstants.propagationDelaySecsStr=1"
echo "  -X github.com/filecoin-project/lotus/build/buildconstants.equivocationDelaySecsStr=0"
