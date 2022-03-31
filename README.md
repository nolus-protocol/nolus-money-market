# Smart Contracts

Implementation of the core business logic as cosmwasm contracts.

## Recommended user's workspace

### Setup

1. Install the Rust Toolchain Installer

    * follow the instructions on https://rustup.rs/, or
    * **[preferred]** install through your system's package manager, e.g. on ArchLinux use `sudo pacman -S rustup`

2. Install the `wasm32` target

```
rustup target install wasm32-unknown-unknown
```
3. Install the Rust linter

```
rustup component add clippy
```

4. Install a set of handy tools

```
cargo install cargo-edit cargo-workspaces cargo-expand
```

### Test
Run the following in a package directory or on the workspace root.
```
cargo test
```

### Build

* A non-optimized version of a contract, run in a contract directory:
```
cargo build --target=wasm32-unknown-unknown
```

* An optimized and verifiable version of all contracts, run on the workspace directory:
```
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/workspace-optimizer:0.12.5
```
[Ref](https://github.com/CosmWasm/rust-optimizer#mono-repos)

### Deploy smart contract

* Add new key to be used for the deployment:
```
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

```
nolusd keys show -a treasury
> nolus122f36dx292yy72253ufkt2g8rzheml2pkcfckl
```


Use the treasury address to send tockens to the new "wallet" account

```
nolusd query bank total $NODE
nolusd tx bank send nolus122f36dx292yy72253ufkt2g8rzheml2pkcfckl nolus1um993zvsdp8upa5qvtspu0jdy66eahlcghm0w6 1000000unolus --chain-id nolus-local --keyring-backend test
nolusd query bank balances nolus1um993zvsdp8upa5qvtspu0jdy66eahlcghm0w6 --chain-id nolus-localnolus-local
```

* set environment
```
export CHAIN_ID="nolus-local"
export TXFLAG="--chain-id ${CHAIN_ID} --gas-prices 0.025unolus --gas auto --gas-adjustment 1.3"
```

* see how many codes we have now
```
nolusd query wasm list-code
```

* now we store the bytecode on chain; you can see the code in the result
```
RES=$(nolusd tx wasm store artifacts/<contract name>.wasm --from wallet $TXFLAG -y --output json -b block)
```

* you can also get the code this way
```
CODE_ID=$(echo $RES | jq -r '.logs[0].events[-1].attributes[0].value')
```

* no contracts yet, this should return an empty list
```
nolusd query wasm list-contract-by-code $CODE_ID --output json
```

* you can also download the wasm from the chain and check that the diff between them is empty
```
nolusd query wasm code $CODE_ID download.wasm
diff artifacts/<contract name>.wasm download.wasm
```

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


# Resources
## References
- [Rust Language](https://doc.rust-lang.org/reference/index.html)
- [Notable Traits in the Standard Library](https://github.com/pretzelhammer/rust-blog/blob/master/posts/tour-of-rusts-standard-library-traits.md)
- My favourite and all-in-one is [Rust Language Cheat Sheet](https://cheats.rs/)

## Rust and WASM Tutorials
- Many resources are linked at [Learn Rust](https://www.rust-lang.org/learn)
- [The official book](https://doc.rust-lang.org/book/)
- [Cooking with Rust](https://rust-lang-nursery.github.io/rust-cookbook/about.html)
- [Learning Rust by programming](https://rust-unofficial.github.io/too-many-lists/)
- [Rust by Examples](https://doc.rust-lang.org/rust-by-example/)
- [Style Guidelines](https://doc.rust-lang.org/1.0.0/style/) although partially completed
- [API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Rust Design Patterns](https://rust-unofficial.github.io/patterns/)
- A collection of resources to guide programmers to write [Idiomatic Rust code](https://github.com/mre/idiomatic-rust)
- [Node to Rust series](https://vino.dev/blog/node-to-rust-day-1-rustup/)
- Advanced concepts like ownership, type conversions, etc [The Rustonomicon](https://doc.rust-lang.org/stable/nomicon/index.html)
- A nice collection of [selected posts](https://github.com/brson/rust-anthology/blob/master/master-list.md)

## Rust and Blockchains
- [Terra Academy](https://academy.terra.money/courses/cosmwasm-smart-contracts-i)
