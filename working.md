# Setup env vars
```bash
export FOC_CONTRACT_USDFC=0xB514FeE11119E0923950C09A181F1fa3aa62C80b ;
export FOC_CONTRACT_FWSS=0x4A8a81765bFBe09D6fDd167EF954a1D3401340e5 ;
export FOC_CONTRACT_MULTICALL=0x2e1F1424b41ad7b2E34b0a60501edFc82FEf5BE8 ;
export FOC_CONTRACT_SIMPLE=0x0000000000000000000000000000000000000000 ;
export FOC_CONTRACT_PAY=0xFD61fA68CB8F70dfC35a4AB244703e39BaB9F352 ;
export CURIO_DB_HOST=localhost;
export CURIO_DB_PORT=5703 ;
export CURIO_DB_CASSANDRA_PORT=5704;
export CURIO_DB_USER=yugabyte ;
export CURIO_DB_PASSWORD=yugabyte ;
export CURIO_DB_NAME=yugabyte ;
export CURIO_DB_LOAD_BALANCE=false;
export FULLNODE_API_INFO=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJBbGxvdyI6WyJyZWFkIiwid3JpdGUiLCJzaWduIiwiYWRtaW4iXX0.-A_dOiryIy0L91-CkYi8vedSLEAfiKuPhN-21ijJX_I:/dns4/localhost/tcp/5700/http
export LOTUS_PATH=/home/redpanda/.foc-localnet/artifacts/docker/volumes/lotus-data;
export PDP_PRIVATE_KEY=cd3ec679d4c6928ff7db1854ee1720ad0e0f8c03299c093602360c431452705c;
```

# First run, setting up new-cluster
curio config new-cluster t01001

(finishes by itself)

# Setup PDP layer

curio config create --title pdp-only << 'EOF'
[HTTP]
DelegateTLS = true
DomainName = "pdp-sp-0.foc-localnet.internal"
Enable = true
ListenAddress = "0.0.0.0:4702"

[Subsystems]
EnableCommP = true
EnableMoveStorage = true
EnablePDP = true
EnableParkPiece = true
EOF

Layer pdp-only created/updated

# Attach storage

curio run --nosync --layers seal,post,pdp-only,gui

Wait till localhost:4701 is available

curio cli storage attach --init --seal /home/redpanda/.foc-localnet/artifacts/docker/volumes/curio/fast-storage
curio cli storage attach --init --store /home/redpanda/.foc-localnet/artifacts/docker/volumes/curio/long-term-storage


# PDPTool Magic

┌─────────────────────────────────────────────────────────────────┐
│ CLIENT SIDE (pdptool)                                           │
└─────────────────────────────────────────────────────────────────┘

1. pdptool create-service-secret
   ├─ Generates ECDSA P-256 key pair
   ├─ Saves PRIVATE key → pdpservice.json (keep secret!)
   └─ Outputs PUBLIC key → Register with Curio server

2. pdptool create-jwt-token "pdp"
   ├─ Loads private key from pdpservice.json
   ├─ Creates JWT claims:
   │   {
   │     "service_name": "pdp",
   │     "exp": 1734480000  // 24 hours from now
   │   }
   ├─ Signs with ES256 algorithm using PRIVATE key
   └─ Outputs JWT token: "eyJhbGciOiJFUzI1NiIs..."

3. Send HTTP request with JWT:
   Authorization: Bearer eyJhbGciOiJFUzI1NiIs...


pdptool create-service-secret > pdp_service_pubkey.txt
Saves:
    - Pubkey in pdp_service_pubkey.txt
    - PrivKey in PDPService.json


Give on-chain signing key to curio (PDP_SP_0 key):
```bash
curl -X POST -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"method\":\"CurioWeb.ImportPDPKey\",\"params\":[\"cd3ec679d4c6928ff7db1854ee1720ad0e0f8c03299c093602360c431452705c\"],\"id\":1}" \
    http://localhost:4701/api/webrpc/v0
```
result would be pubkey of the PDP_SP_0, verify it
type: `{"id":1,"jsonrpc":"2.0","result":"0x1988D2De200fD1aEC55931376e74073d90f64DAC"}`

Use PDPTool output pubkey and send it over to curio, payload looks like this:
```json
{
  "jsonrpc": "2.0",
  "method": "CurioWeb.AddPDPService",
  "params": [
    "pdp",
    "-----BEGIN PUBLIC KEY-----\nMFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEGSPt+D+keBx6vPVUEoPrzFtwYf2+\nBEUcKynWk0u9iOfiV8OO5vkIECGEfDWKh7kmPcndcff6p44OWP0Z+j7Jpw==\n-----END PUBLIC KEY-----"
  ],
  "id": 2
}
```

So, sending looks like:
```bash
echo "Creating service tokens..."
pdptool create-service-secret > pdp_service_key.txt

echo "Creating JWT token..."
pdptool create-jwt-token pdp | grep -v "JWT Token:" > jwt_token.txt

# Extract public key from the output and properly format it
PUB_KEY=$(cat pdp_service_key.txt | sed -n '/Public Key:/,/-----END PUBLIC KEY-----/p' | grep -v "Public Key:" | sed 's/^[[:space:]]*//')
echo "Public Key (formatted):"
echo "$PUB_KEY"

JSON_PUB_KEY=$(echo "$PUB_KEY" | awk '{printf "%s\\n", $0}' | sed 's/\\n$//')
  curl -X POST -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"method\":\"CurioWeb.AddPDPService\",\"params\":[\"pdp\",\"$JSON_PUB_KEY\"],\"id\":2}" \
    http://localhost:4701/api/webrpc/v0
```
output would be: {"id":2,"jsonrpc":"2.0","result":null}

Ensure connectivity:
```
# Test connectivity to the PDP service endpoint
echo "Testing PDP connectivity..."
pdptool ping --service-url http://localhost:4702 --service-name pdp
```
It should return "Ping successful: Service is reachable and JWT token is valid."






---------------------
We need to re-implement CurioStep to setup Curio SP properly, alongwith other steps.

Create a temporary task tracker `upgrades.md` to track between AI runs.
After each milestone, add git commits

- First, we need to accomodate for the fact that there will be multiple curio processes down the line.
    - As such those nodes will be called 0, 1, 2, 3 etc
    - For example, currently addresses are `PDP_SP_0` as the sole PDP service provider. This will be extended later, and made base 1.
    - Actionable: Create as many miners pre-sealed sectors (currently we have only 2, one for lotus-miner, one for curio) as there are pdp service providers. So net, there will be one lotus-miner, and N pdp service provider pre-sealed sectors.
    - Actionable: Create upto 5 (constant add: MAX_PDP_SP_COUNT) PDP SP addresses during genesis / key generation process. They will be PDP_SP_1. through PDP_SP_5 (base 1 not base 0). They may or may not be used in CurioStep, but their keys are generated.
    - Actionable: Spawn as many `foc-yugabyte` containers as number of PDP service providers. They can be suffixed with -1, -2 etc. Each will be on separate yugabyte networks. (again, base 1)
    - All `cargo run -- start --reset` should delete `~/.foc-localnet/artifacts/docker/volumes/curio/*` volumes

CurioStep needs to be overhauled. It does too little and post-execute verifications are not proper.
New CurioStep will:
- have multi-file module with each file no longer than 250 lines of code
- May spawn up multiple curio node, currently only spawns 1, but should be configurable via config.toml in ~/.foc-localnet
- Each curio node:
    - Pre-execute: verifies that lotus is running and blocks are being generated, similar to lotus-miner.
    - Execute: Is a complex multi-step process for each curio SP node documented below.
    - Post-execute: verifies that we can ping PDP endpoint properly and piece upload and download works properly via pdptool (no JWT tokens)


## curio execute

- Each curio sp (with identifier X) should depend on volumes `~/.foc-localnet/artifacts/docker/volumes/curio/X/`
- Curio needs to run first inside `foc-curio` for base DB migration (DB migration on base layer). Each curio SP has their own yugabyte `foc-yugabyte`
    - curio new-cluster "t01001" etc.
    - Sets up curio "base" layer
    - The way this is done is via: `curio config new-cluster <miner_id>` example: `curio config new-cluster t01001`
    - This does not start a daemon, but just sets up db and finishes when that is done.
- Provide curio 'pdp-layer' configs. It can be done via:
    ```
    curio config create --title pdp-only << 'EOF'
    [HTTP]
    DelegateTLS = true
    DomainName = "pdp-sp-0.foc-localnet.internal"
    Enable = true
    ListenAddress = "0.0.0.0:4702"

    [Subsystems]
    EnableCommP = true
    EnableMoveStorage = true
    EnablePDP = true
    EnableParkPiece = true
    EOF
    ```
    - setups up 'pdp-layer'
    - we don't care about domain-name. anything can be put there
    - DelegateTLS should be true always.
    - This does not start a daemon, but just sets up db and finishes when that is done.
- Start curio daemon with `curio run --nosync --layers seal,post,pdp-only,gui`
    - This spawns up curio daemon. However, curio does not know how to deal with storage right now.
    - Tell curio to use certain "storage" locations. You can execute the following commands to setup
        - "fast-storage"
        - "long-term-storage"
        - Commands:
        ```sh
        curio cli --machine 127.0.0.1:12300 storage attach \
            --init \
            --seal \
            --weight 10 \
            /home/redpanda/.foc-localnet/artifacts/docker/volumes/curio/X/fast-storage
        curio cli --machine 127.0.0.1:12300 storage attach \
            --init \
            --store \
            --weight 10 \
            /home/redpanda/.foc-localnet/artifacts/docker/volumes/curio/X/long-term-storage
        ```
        - However, this will not be used verbatim because we will be mounting into docker the host volumes
            - Most probably these will be mounted into `/home/foc-user/curio/fast-storage` and `/home/foc-user/curio/long-term-storage`
    - Tell curio about PDP private key it needs to use to communicate on-chain
        ```bash
        curl -X POST -H "Content-Type: application/json" \
            -d "{\"jsonrpc\":\"2.0\",\"method\":\"CurioWeb.ImportPDPKey\",\"params\":[\"<pdp-sp-x-ethereum-private-key>\"],\"id\":1}" \
            http://localhost:4701/api/webrpc/v0 
        ```
        - This presents back output of the format: `{"id":1,"jsonrpc":"2.0","result":"0x1988D2De200fD1aEC55931376e74073d90f64DAC"}`
        - The result back is the public key / address of pdp-sp-x. this needs to be verified if they are same as what is mentioned in addresses.json


## curio post-execute

1. Check if we can ping PDP services (use reqwest I guess?):
    Check PDP subsystem is working (handlers are running?):
    curl -X GET http://localhost:4702/pdp/ping

    Is the local curio storage roughly works (pull in PDPTool upload-file)?

2. Check if storage, upload and download works
    - first create a random file with random data (worth 1Kb)
    - Upload it to curio via pdptool:
    ```sh
    pdptool upload-piece \
        --service-url http://localhost:4702 \
        --service-name public \
        --hash-type commp \
        README.md \
        --verbose
    ```
    - this does not output anything. but we run the same command again to get a Piece CID.
    - output would be of format:
    ```sh
    http.StatusOK
    Piece already exists on the server. Piece CID: bafkzcibdyeoaqt65bbxel7udfzz676ar44327o527ugh2seukeme7anlcpi23rqq
    Piece uploaded successfully.
    ```
    - use this piece CID (`bafkzcibdyeoaqt65bbxel7udfzz676ar44327o527ugh2seukeme7anlcpi23rqq`) to download the file via (use reqwest):
        - curl -X GET http://localhost:4702/piece/bafkzcibdyeoaqt65bbxel7udfzz676ar44327o527ugh2seukeme7anlcpi23rqq
        - verify if the data retrieved is what was uploaded
