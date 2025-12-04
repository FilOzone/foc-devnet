# How to deploy Multicall3
- Use pre-signed deployment tx available at `signed_tx.dat`. 
- It used the deployer address given at `deployer_multicall3.txt` to be pre-funded.
- Move some funds from `DEPLOYER_MULTICALL3` to the address given at: `deployer_multicall3.txt`
- Cast publish transaction `cast publish $TX --rpc-url $RPC_URL`
- ABI for interacting with multicall3 is available at `multicall3.abi.json`. Verify that we can call some functions via this, and that contract works.