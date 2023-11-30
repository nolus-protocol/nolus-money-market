# Nolus Money Market

<br /><p align="center"><img alt="nolus-money-market-logo" src="docs/money-market-logo.svg" width="100"/></p><br />

Implementation of the core business logic as CosmWasm contracts.

## Recommended user workspace

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

The build is controlled with a few environment variables:

* `RELEASE_VERSION` - an arbitrary string giving the release a name
* `NET_NAME` - sets the targeted network; possible values are:
  * `dev`
  * `test`
  * `main`
* `PROTOCOL_NAME` - sets the targeted protocol

#### Workspaces

Th project is separated into three workspaces:

* `platform` - Network- and protocol-agnostic contracts and packages
* `protocol` - Network- and protocol-specific contracts and packages
* `tests` - integration tests

In the instructions below this value is stored in *WORKSPACE_DIR_NAME*.

#### A non-optimized version

The command below builds a contract if ran from the contract directory,
or builds the contracts of the workspace from within which it is ran:

```sh
RELEASE_VERSION='dev-release' NET_NAME='dev' PROTOCOL_NAME='osmosis' cargo build --features "net_${$NET_NAME},${PROTOCOL_NAME}" --target=wasm32-unknown-unknown
```

One way to avoid having to set those environment variables is to
set them in the editor/IDE's configuration.

An example one for VSCode/VSCodium, located at `.vscode/settings.json`, is shown here:

```json
{
  "rust-analyzer.cargo.extraEnv": {
    "RELEASE_VERSION": "local",
  },
  "terminal.integrated.env.linux": {
    "RELEASE_VERSION": "local",
  },
  "terminal.integrated.env.osx": {
    "RELEASE_VERSION": "local",
  },
  "terminal.integrated.env.windows": {
    "RELEASE_VERSION": "local",
  },
}
```

#### An optimized version

##### Container image

First, the image for building the contracts needs to be built. This happens by
running the command shown here:

```sh
docker build . -f "Containerfile" -t "wasm-optimizer" --build-arg "rust_ver=1.72"
```

Do note that the command is an example one and the Rust version, denoted by the
`rust_ver` build argument, should match the one set in the `rust-toolchain.toml`
file, located at the root of the project!

##### Running container image

The command shown below builds an optimized and verifiable version of
each set of contracts, depending on their workspace (indicated by
`WORKSPACE_DIR_NAME`), the targeted protocol (indicated by `PROTOCOL`) and targeted network
(indicated by `NET`):

* ```sh
  export WORKSPACE_DIR_NAME='platform'
  ```

  OR

  ```sh
  export WORKSPACE_DIR_NAME='protocol'
  ```

* ```sh
  export NET='dev'
  export PROTOCOL='osmosis'
  export ARTIFACTS_SUBDIR="$([[ $WORKSPACE_DIR_NAME == 'protocol' ]] && echo $PROTOCOL || echo 'platform')"
  ```

  ```sh
  mkdir -p "$(pwd)/artifacts/${ARTIFACTS_SUBDIR}/" && \
    docker run --rm -v "$(pwd)/platform/:/platform/" \
    -v "$(pwd)/protocol/:/protocol/" \
    -v "$(pwd)/${WORKSPACE_DIR_NAME}/:/code/" \
    -v "$(pwd)/artifacts/${ARTIFACTS_SUBDIR}/:/artifacts/" \
    --env "RELEASE_VERSION=`git describe`-`date -Iminute`" \
    --env "features=cosmwasm-bindings$(if test "${WORKSPACE_DIR_NAME}" = 'protocol'; then echo ",net_${NET}"; fi)$(if test "${WORKSPACE_DIR_NAME}" = 'protocol'; then echo ",${PROTOCOL}"; fi)"
    wasm-optimizer
  ```

**NOTE:** As one might set those environment variables in the settings
of their editor/IDE, those environment variables still must be provided
as arguments to the `docker run` command.
Exception to this should be the `platform` workspace, as it strives to
be agnostic to the targeted network and protocol.

**NOTE:** Builds are reproducable *as long as* all environment variables
passed to the container are the exact same. If it is desired to build
a verification copy of the contracts, one must set the `RELEASE_VERSION`
environment variable to the one used to build the original instead.

### Test

Run the following in a package directory or any workspace.

```sh
cargo test --features "net_${NET},${PROTOCOL}" --all-targets
```

### Lint

Run the following in the `platform` workspace.

```sh
./lint.sh
```

Run the following in the `protocol` and `tests` workspaces.

```sh
./lint.sh "net_${NET},${PROTOCOL}"
```

### New contracts - genesis

Contract addresses are dependent on the order in which they are deployed in the script.

When adding a new contract, and it needs to be deployed with the genesis:

1. Add it to the `scripts/deploy-contracts-genesis.sh` script.
2. Ensure you preserve the order:
    * Your contract **is not** a dependency:
      * Add your initialization logic at the end and fill in the address that you
        get based on the contract's ID.
    * Your contract **is** a dependency:
      * Find the position corresponding to the contract's position in the dependency tree.
      * Assume the address of the first contract that you pushed down.
      * **Shift** down the addresses of the following contracts.

        In the end, you should be left with one contract for which there won't
        be an address to assume.
      * After you are done with the address shifting, fill out the address of the
        contract in the script file, which you get based on the contract's ID.

#### Reordering contracts because one is now a dependency

As mentioned in the section above, contract addresses are dependent on the order
in which they are deployed in the script.

When changing the order of deployment, reorder the contracts' addresses accordingly,
so the order of the actual addresses is **not** changed but the contract who owns
that address is.

### New contracts - live network

The process of deploying a new contract on a live network is presented with the steps below:

#### Upload the contract code

```sh
nolusd tx wasm store <wasm_file_path> --instantiate-anyof-addresses <addresses_to_instantiate_the_code>  --from <store_code_privileged_user_key>
```

#### Get the expected contract address

Due to the fact that contract addresses depend on the order in which they are deployed, and because of the dependencies between some of their init messages, the new contract address must be predicted. Тherefor, there is a query provided by the `admin` contract:

```sh
nolusd q wasm contract-state smart <admin_contract_address> '{"instantiate_address":{"code_id":<code_id_from_the_previous_step>,"protocol":"<protocol>"}}'
```

Where <`protocol`> is a combination of the chosen DEX name and the protocol currency (eg "osmosis-USDC").

#### Instantiate the contract

On a live network, a new contract can be instantiated through the `admin` contract:

```sh
nolusd tx wasm execute <admin_contract_address> '{"instantiate":{"code_id":<code_id>,"label":"<label>","message":"<init_msg>","protocol":"<protocol>","expected_address":"<expected_address_received_from_the_previous_step>"}}' --from <network_DEX_admin_key>
```

Where <`label`> can be a combination of the chosen protocol and the contract name (eg `osmosis-USDC-leaser`)

If the given expected address matches the real one, the instantiation will be successful.

### Deploy new Protocol-specific contracts

#### Deploy new contracts

As mentioned in the sections above, the order in which contracts are deployed is important. So there is a correct way to deploy a new set of Protocol-specific contracts.

1. store Leaser code
2. store Lease code
3. store and instantiate LPP
4. store and instantiate Oracle
5. store and instantiate Profit
6. instantiate Leaser

This can be done manually by following the steps in the section [above](#new-contracts---live-network),
or by using the `deploy-contracts-live.sh`:

```sh
./scripts/deploy-contracts-live.sh deploy_contracts "<nolus_node_url>" "<nolus_chain_id>" "<nolus_home_dir>" "<network_DEX_admin_key>" "<store_code_privileged_user_key>" "<admin_contract_address>" "<protocol_wasm_artifacts_dir_path>" "<dex_name>" "<protocol_currency>" "<treasury_contract_address>"  "<timealarms_contract_address>" '<protocol_swap_tree_obj>'
```

#### Register the new set of Protocol-specific contracts

The goal is to make the platform to work with the new contracts as well.

```sh
nolusd tx wasm execute <admin_contract_address> '{"register_protocol":{"name":"<protocol>","contracts":{"leaser":"<leaser_contract_address>","lpp":"<lpp_contract_address>","oracle":"<oracle_contract_address>","profit":"<profit_contract_address>"}}}' --from <network_DEX_admin_key>
```

### Upgrade dependencies

Using the previously installed cargo-edit one can easily upgrade the dependencies.

For more details please refer to

```sh
cargo upgrade --help
```

An example:

```sh
cargo upgrade --workspace cw-storage-plus
```

[Ref](https://github.com/CosmWasm/rust-optimizer#mono-repos)

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
* Advanced concepts like ownership, type conversions, etc. [The Rustonomicon](https://doc.rust-lang.org/stable/nomicon/index.html)
* A nice collection of [selected posts](https://github.com/brson/rust-anthology/blob/master/master-list.md)

### Rust and Blockchains

* [Terra Academy](https://academy.terra.money/courses/cosmwasm-smart-contracts-i)
