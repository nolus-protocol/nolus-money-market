# Nolus Money Market
<p align="center">
<img alt="nolus-money-market-logo" src="docs/money-market-logo.svg" width="100" />
</p>
Implementation of the core business logic as CosmWasm contracts.

# Prerequisites
* OCI-compatible (Open Container Initiative) container engine, e.g. Docker CE,
  Podman, Nerdctl, etc.  
  For simplicity, Docker CLI will be assumed, but it can be easily aliased for
  Podman and Nerdctl as they are CLI-compatible for the most part.

# Developer setup
1. Install the Rust Toolchain Installer
   * follow the instructions on [https://rustup.rs/](https://rustup.rs/), or
   * **[preferred]** install through your system's package manager, e.g.
    on ArchLinux use `sudo pacman -S rustup`
2. Install the stable Rust toolchain
   ```sh
   rustup toolchain install --profile=default --target=wasm32-unknown-unknown stable
   ```
3. Install the nightly Rust toolchain  
   Selected profile is left at the developer's discretion.
   ```sh
   rustup toolchain install nightly-2024-02-05
   ```
4. \[Optional\] Install a set of handy tools
   ```sh
   cargo install cargo-edit cargo-workspaces cargo-expand
   ```

# Workspaces
The project is separated into three workspaces:
* `platform` - Network- and protocol-agnostic contracts and packages
* `protocol` - Network- and protocol-specific contracts and packages
* `tests` - Blackbox integration tests
* `tests` - Tools used within the project

From now on the instructions will assume the workspace name is stored in
`WORKSPACE_DIR_NAME`.

Building in the following workspaces requires setting the `RELEASE_VERSION`
environment variable:
* `platform/contracts`
* `protocol/contracts`
* `tests`
* And possibly packages from `*/packages` subdirectories which might be
  referencing contracts directly or indirectly.

The `RELEASE_VERSION` environment variable gives the built contracts an
arbitrary string by which the pack of contracts is distinguished from others.

## \[Optional\] Additional developer setup
One way to avoid having to set the environment variables is to set it in the
editor/IDE's configuration.  
An example one for VSCode/VSCodium, located at `.vscode/settings.json`,
is shown here:
```json
{
  "rust-analyzer.cargo.extraEnv": {
    "RELEASE_VERSION": "dev"
  },
  "terminal.integrated.env.linux": {
    "RELEASE_VERSION": "dev"
  },
  "terminal.integrated.env.osx": {
    "RELEASE_VERSION": "dev"
  },
  "terminal.integrated.env.windows": {
    "RELEASE_VERSION": "dev"
  }
}
```

# Development
## Required project tooling
* `crates.io/crates/cargo-audit`
  * Used for auditing dependencies.
  * Installation: `cargo install cargo-audit`
* `crates.io/crates/cargo-udeps`
  * Used for checking for unused dependencies.
  * Installation: `cargo install cargo-udeps`
* `crates.io/crates/cosmwasm-check`
  * Used for checking compiled WebAssembly binary artifacts.
  * Installation: `cargo install cosmwasm-check`
* `./tools/cargo-each`
  * Used for proxying commands, both `cargo` subcommands and external ones.
  * Installation: `cargo install --path ./tools/cargo-each`

## Development
During development, to run checks & the linter, the following templates should
be used:
* Basic version
  ```sh
  cargo -- each --manifest-path "./${WORKSPACE_DIR_NAME}/Cargo.toml" run --tag ci -- <...>
  ```
* Shorthand version:
  ```sh
  cargo each --manifest-path "./${WORKSPACE_DIR_NAME}/Cargo.toml" run -t ci -- <...>
  ```
* Version printing the command to be executed:
  ```sh
  cargo each --manifest-path "./${WORKSPACE_DIR_NAME}/Cargo.toml" run --tag ci --print-command -- <...>
  ```
* Version executing non-Cargo command:
  ```sh
  cargo each --manifest-path "./${WORKSPACE_DIR_NAME}/Cargo.toml" run --external-command --tag ci -- <...>
  ```

These templates can be mixed in order to get the desired combination of
features. `<...>` is left as a placeholder for the actual command.

Valid replacements for the `<...>` placeholder are:
* `check`
  * Runs `cargo check` with each configured features combination.
* `./lint.sh --profile "${PROFILE}" --not-as-workspace`
  * `--external-command` has to be set
  * Runs the linter with each configured features combination while also setting
  which profile, chosen through the `PROFILE` environment variable, should be
  used.
* `test`
  * Runs `cargo test` with each configured features combination.

# Release
## Builder-&-optimizer container image
To build the container image the following command should be used:
```sh
docker build --pull --file "Containerfile" --tag "wasm-optimizer:${RUST_VERSION}" --build-arg "rust_ver=${RUST_VERSION}" .
```

In the command, the desired version of the [`docker.io/library/rust`](https://hub.docker.com/_/rust) image
is set via the `RUST_VERSION` environment variable. It can be substituted with
`latest`.  
Instructions from here on will reference that environment variable.

## Running the container
Running the container requires binding the target workspace as `/code/` within
the container itself.  
When the target workspace is `./protocol`, `./platform` needs to be bound to the
container too, under `/platform/`.

The output produced by the container is stored under a path declared as a
volume. That means that when not bound, the container engine will create an
anonymous one.  
While it is possible to copy the artifacts out of the volume later, it is
recommended that a well-known volume or a host directory is bound.  
Instructions from now on will assume that a host directory shall be bound.

Due to the nature of the container engines, the output artifacts directory has
to be created before running the container. It can be any arbitrary directory of
the user's choosing.

**BEWARE:** The output artifacts directory is always cleaned before starting the
compilation of the contracts. Make sure to always use a different directory for
the different targets, if keeping the ones already existing in the directory is
required.

As noted before, the `RELEASE_VERSION` environment variable is required in order
to proceed with the compilation. Automated release builds will use the output of
Git's `describe` command and concatenate it with the current date and time, with
precision in minutes, as follows: `$(git describe)-$(date -Iminute)`.

The container is created with the following command:
```sh
docker container create --volume "$(pwd)/${WORKSPACE_DIR_NAME}/:/code/:ro" --volume "${ARTIFACTS_SUBDIR}/:/artifacts/:rw" --env "RELEASE_VERSION=${RELEASE_VERSION}" "wasm-optimizer:${RUST_VERSION}"
```
As noted before, when the target workspace is `./protocol`, `./platform` needs to
also be bound. This happens by adding an additional `--volume` flag as
follows:  
`--volume "$(pwd)/platform/:/platform/:ro"`

Additionally, a name can be specified via adding the `--name` flag followed by
the chosen name.

# Managing contracts
## New contracts - genesis
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

#### Read the expected contract address

Due to the fact that contract addresses depend on the order in which they are deployed, and because of the dependencies between some of their init messages, the new contract address must be predicted. Тherefor, there is a query provided by the `admin` contract:

```sh
nolusd q wasm contract-state smart <admin_contract_address> '{"instantiate_address":{"code_id":<code_id_from_the_previous_step>,"protocol":"<protocol>"}}'
```

Where <`protocol`> is a combination of the chosen DEX name and the protocol currency (eg "osmosis-osmosis-usdc_axelar").

#### Instantiate the contract

On a live network, a new contract can be instantiated through the `admin` contract:

```sh
nolusd tx wasm execute <admin_contract_address> '{"instantiate":{"code_id":<code_id>,"label":"<label>","message":"<init_msg>","protocol":"<protocol>","expected_address":"<expected_address_received_from_the_previous_step>"}}' --from <network_DEX_admin_key>
```

Where <`label`> can be a combination of the chosen protocol and the contract name (eg `osmosis-osmosis-usdc_axelar-leaser`)

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
./scripts/deploy-contracts-live.sh deploy_contracts "<nolus_node_url>" "<nolus_chain_id>" "<nolus_home_dir>" "<network_DEX_admin_key>" "<store_code_privileged_user_key>" "<admin_contract_address>" "<protocol_wasm_artifacts_dir_path>" "<dex_network>" "<dex_name>" "<dex_connection>" "<dex_channel_local>" "<dex_channel_remote>" "<protocol_currency>" "<treasury_contract_address>"  "<timealarms_contract_address>" '<protocol_swap_tree_obj>'
```

#### Register the new set of Protocol-specific contracts

The goal is to make the platform to work with the new contracts as well.

```sh
nolusd tx wasm execute <admin_contract_address> '{"register_protocol":{"name":"<protocol>","protocol":{"network":"<network>","contracts":{"leaser":"<leaser_contract_address>","lpp":"<lpp_contract_address>","oracle":"<oracle_contract_address>","profit":"<profit_contract_address>"}}}}' --from <network_DEX_admin_key>
```

#### Read protocol-specific contract addresses

Read all protocols:

```sh
nolusd q wasm cs smart <admin_contract_address> '{"protocols":{}}'
```

Read protocol:

```sh
nolusd q wasm cs smart <admin_contract_address> '{"protocol":{"protocol":"<protocol_name>"}}'
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

### VSCode/VSCodium

Add Rust support by installing `rust-analyzer` extension

1. Press `Ctrl+Shift+P`
2. Execute `ext install rust-lang.rust-analyzer`

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
