sudo run -rf ~/.foc-localnet;

cargo run -- init \
    --curio local:/home/redpanda/code/curio \
    --lotus local:/home/redpanda/code/lotus \
    --force

cargo run -- build curio

---------------

Now that we have `start` command and `stop` command, let's start implementing the crux of localnet setup.

You might need to create a whole lot of things, so break this into small parts. Also, ask clarifying questions where you need.
Wherever required, attempt to create modules which are folder like with mod.rs, file.rs etc.
Split functionality across files in module.
Provide ample documentation.

`foc-builder` is a multi-use container used for building, running ad-hoc code, for smart contract deployment etc.
`foc-curio`, `for-lotus` etc are lean... long running containers hosting miners and exection nodes. Try alpine for them.

The following needs to be started in order:
- `lotus`, execution node, running FEVM and FVM
- `lotus-miner`, first generation miner node building tipsets and doing PoRep (original filecoin)
- `yugabyte`, a postgres like database needed by curio
- `curio`, second generation miner node not building tipsets but doing PDP

Obviously if any of them fail while starting, the start process needs to stop any already running containers.

- None of these containers access internet. 
- They expose to the host all their ports
- They need to interconnect. `yugabyte` is only for `curio` for example.

-----

`lotus` needs to fetch params from the internet to execute the chain. Running a localnet is defined here, use this for more context:
Web link: https://lotus.filecoin.io/lotus/developers/local-network/

Firstly, lotus needs to be built using a special command:
`make 2k`. This should be seen in `run_build_in_container`

## Downloading lotus parameters
Once above is done, lotus needs some extra parameters to run and those can be fetched via
`lotus fetch-params 2048`. 

This downloads a bunch of files in `/var/tmp/filecoin-proof-parameters/`. Make this a constant in code somewhere and run this (inside builder container) in case such file is not available in `~/.foc-localnet/artifacts/filecoin-proof-parameters/`. This directory needs to be loaded in the `foc-lotus` container when it is running at `/var/tmp/filecoin-proof-parameters/`.

There is no dockerfile for this, we need to build one.

## Important `lotus-*` build artifacts
It seems that we currently only depend on `lotus` and `lotus-miner` being important build artifacts. However, it seems like:
- `lotus-shed`: helps make BLS keypairs and addresses, compatible with filecoin
- `lotus-seed`: creates genesis

are also imporant. Add that to binaries directory and let `status` subcommand show its status as well.

## Create two keys with `lotus-shed`
Create two keys with `lotus-shed` binary inside the `foc-builder` container. They need to be created via:
```
./lotus-shed keyinfo new bls
```
We should have yet another two docker volumes called `lotus-keys-1` and `lotus-keys-2` where these keys will be stored.

## Pre-sealing
Before we start a new network, we need to "pre-seal" two sectors for genesis block:
Use in `foc-builder`:
```
./lotus-seed pre-seal --sector-size 2KiB --num-sectors 2
```
Obviously, their output needs to be put in yet another docker volume. Let's call that `genesis-presealed-sectors`.