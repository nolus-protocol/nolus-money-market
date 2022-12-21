# Smart Contracts

<br /><p align="center"><img alt="nolus-money-market-logo" src="docs/money-market-logo.svg" width="100"/></p><br />

Implementation of the core business logic as cosmwasm contracts.

## Recommended user's workspace

### Setup

1. Install the Rust Toolchain Installer

    * follow the instructions on [https://rustup.rs/](https://rustup.rs/), or
    * **[preferred]** install through your system's package manager, e.g. on ArchLinux use `sudo pacman -S rustup`

2. Install the `wasm32` target

```sh
rustup target install wasm32-unknown-unknown
```

3. Install the Rust linter

```sh
rustup component add clippy
```

4. Install a set of handy tools

```sh
cargo install cargo-edit cargo-workspaces cargo-expand
```

### Build

* A non-optimized version of a contract, run in a contract directory:

```sh
cargo build --target=wasm32-unknown-unknown
```

* An optimized and verifiable version of all contracts, run on the workspace directory:

```sh
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/workspace-optimizer:0.12.10
```

### Test

Run the following in a package directory or on the workspace root.

```sh
cargo test
```

### Lint

Run the following in the workspace root.

```sh
./lint.sh
```

### New contracts

Contract's addresses are dependent on the order in which they are deployed in the script.

When adding a new contract, and it needs to be deployed with the genesis:
1. Add it to the `scripts/deploy-contracts-genesis.sh` script.
2. Ensure you preserve the order:
    * Your contract **is not** a dependency:
      * Add your initialization logic at the end and fill in the address that you get based on the contract's ID.
    * Your contract **is** a dependency:
      * Find the position corresponding to contract's position in the dependency tree.
      * Assume the address of the first contract that you pushed down.
      * **Shift** down the addresses of the following contracts.

        In the end, you should be left with one contract for which there won't be an address to assume.
      * After you have done with the address shifting, fill in the contract without an address the one you get based on the contract's ID.

### Reordering contracts because one is now dependency

As mentioned in the section above, contract's addresses are dependent on the order in which they are deployed in the script.

When changing the order of deployment, reorder the contracts' addresses accordingly, thus the order of the actual addresses is **not** changed but contract who owns that address is.

### Upgrade dependencies

Using the previously installed cargo-edit one can easily upgrade the dependencies. For more details please refer to 

```sh
cargo upgrade --help
```

An example:

```
cargo upgrade --workspace cw-storage-plus
```

[Ref](https://github.com/CosmWasm/rust-optimizer#mono-repos)

### Deploy smart contract CLI

* Add new key to be used for the deployment:

```sh
nolusd keys add wallet

------------ Example Output-----------------
- name: wallet
  type: local
  address: nolus1um993zvsdp8upa5qvtspu0jdy66eahlcghm0w6
  pubkey: '{"@type":"/cosmos.crypto.secp256k1.PubKey","key":"A0MFMuJSqWpofT3GIQchGyL9bADlC5GEWu3QJHGL/XHZ"}'
  mnemonic: ""
```

* The new key needs some tockens for the deployment

When scripts/init-local-network.sh is started it creates two acconts. One of them is the "treasury" account \
To find the address of the treasury account, run the folloing command:

```sh
nolusd keys show -a treasury
> nolus122f36dx292yy72253ufkt2g8rzheml2pkcfckl
```

Use the treasury address to send tockens to the new "wallet" account

```sh
nolusd query bank total $NODE
nolusd tx bank send $(nolusd keys show -a treasury) $(nolusd keys show -a wallet) 1000000unolus --chain-id nolus-local --keyring-backend test
nolusd query bank balances $(nolusd keys show -a wallet) --chain-id nolus-local
```

* set environment

```sh
export CHAIN_ID="nolus-local"
export TXFLAG="--chain-id ${CHAIN_ID} --gas-prices 0.025unolus --gas auto --gas-adjustment 1.3"
```

* see how many codes we have now

```sh
nolusd query wasm list-code
```

* now we store the bytecode on chain; you can see the code in the result

```sh
RES=$(nolusd tx wasm store artifacts/<contract name>.wasm --from wallet $TXFLAG -y --output json -b block)
```

* you can also get the code this way

```sh
CODE_ID=$(echo $RES | jq -r '.logs[0].events[-1].attributes[0].value')
```

* no contracts yet, this should return an empty list

```sh
nolusd query wasm list-contract-by-code $CODE_ID --output json
```

* you can also download the wasm from the chain and check that the diff between them is empty

```sh
nolusd query wasm code $CODE_ID download.wasm
diff artifacts/<contract name>.wasm download.wasm
```

### Deploy smart contract TypeScript

**We use the [cosmjs](https://www.npmjs.com/package/@cosmjs/cli) library to work with smart contracts via TypeScript.**

First of all, make sure you have created a user who has some amount. We will use this user to upload a contract and send it messages. You can use util/ methods in the UAT-test project to create a new user and client or use existing ones.
Example: the getUserClient() and getUser1Wallet() methods are helpers methods (from UAT-tests/src/util) that do this first step:

```ts
let userClient: SigningCosmWasmClient;
let userAccount: AccountData;
userClient = await getUser1Client();
[userAccount] = await (await getUser1Wallet()).getAccounts();
```

This userAccount address will be transmitted as a sender when we want to send messages to the contract.
Тhe essence of this example shows how to deploy the contract and communicate with it:

1. After building the code of the smart contract, we get the .wasm file we need. The first step is to access this file:

```ts
import * as fs from "fs";

const wasmBinary: Buffer = fs.readFileSync("./oracle.wasm");
```

2. Now we can upload a wasm binary:

```ts
const customFees = {
    upload: {
        amount: [{amount: "2000000", denom: "unolus"}],
        gas: "2000000",
    },
    init: {
        amount: [{amount: "500000", denom: "unolus"}],
        gas: "500000",
    },
    exec: {
        amount: [{amount: "500000", denom: "unolus"}],
        gas: "500000",
    }
};

const uploadReceipt = await userClient.upload(userAccount.address, wasmBinary, customFees.upload);
const codeId = uploadReceipt.codeId;
```

3. Then we can instantiate the contract and get its address:

```ts
const instatiateMsg = {
            "base_asset": "ust",
            "price_feed_period": 60,
            "feeders_percentage_needed": 50,
        };
const contract: InstantiateResult = await userClient.instantiate(userAccount.address, codeId, instatiateMsg, "test", customFees.init);
contractAddress = contract.contractAddress;
```

This **contractAddress** variable is our entry point to the contract. When we send an exacute or query message, we give this address to the methods.

4. How to send a execute message:

```ts
const addFeederMsg = {
            "register_feeder": {
                "feeder_address":"nolus1gzk...."
            },
        };
await userClient.execute(userAccount.address, contractAddress, addFeederMsg, customFees.exec);
```

5. How to send a query message:

```ts
const isFeederMsg = {
            "is_feeder": {
                "address":"nolus1gzk...."
            },
        };
await userClient.queryContractSmart(contractAddress, isFeederMsg);
```

These json messages that we form (including the initial message) depend on what the contract expects to receive in order to provide us with certain functionality. We can check this from the generated json schemas.

### VSCode

Add Rust support by installing `rust-analyzer` extension

1. Press `Ctrl+Shift+P`
2. Execute `ext install matklad.rust-analyzer`

Add syntax highlighting for TOML files

1. Press `Ctrl+Shift+P`
2. Execute `ext install bungcip.better-toml`

Add dependency versions update by installing `crates` extension

1. Press `Ctrl+Shift+P`

2. Execute `ext install serayuzgur.crates`

## Resources

### References

* [Rust Language](https://doc.rust-lang.org/reference/index.html)
* [Notable Traits in the Standard Library](https://github.com/pretzelhammer/rust-blog/blob/master/posts/tour-of-rusts-standard-library-traits.md)
* My favourite and all-in-one is [Rust Language Cheat Sheet](https://cheats.rs/)

### Rust and WASM Tutorials

* Many resources are linked at [Learn Rust](https://www.rust-lang.org/learn)
* [The official book](https://doc.rust-lang.org/book/)
* [Cooking with Rust](https://rust-lang-nursery.github.io/rust-cookbook/about.html)
* [Learning Rust by programming](https://rust-unofficial.github.io/too-many-lists/)
* [Rust by Examples](https://doc.rust-lang.org/rust-by-example/)
* [Style Guidelines](https://doc.rust-lang.org/1.0.0/style/) although partially completed
* [API Guidelines](https://rust-lang.github.io/api-guidelines/)
* [Rust Design Patterns](https://rust-unofficial.github.io/patterns/)
* A collection of resources to guide programmers to write [Idiomatic Rust code](https://github.com/mre/idiomatic-rust)
* [Node to Rust series](https://vino.dev/blog/node-to-rust-day-1-rustup/)
* Advanced concepts like ownership, type conversions, etc [The Rustonomicon](https://doc.rust-lang.org/stable/nomicon/index.html)
* A nice collection of [selected posts](https://github.com/brson/rust-anthology/blob/master/master-list.md)

### Rust and Blockchains

* [Terra Academy](https://academy.terra.money/courses/cosmwasm-smart-contracts-i)
