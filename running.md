sudo run -rf ~/.foc-localnet;

cargo run -- init \
    --curio local:/home/redpanda/code/curio \
    --lotus local:/home/redpanda/code/lotus \
    --force

cargo run -- build curio

---------------

Now that we have `start` command and `stop` command, let's start implementing the crux of localnet setup

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