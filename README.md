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
* `RUSTFLAGS` - set of values that is passed down to the Rust compiler;
  setting targetted network and DEX happens through setting it to:

  ```sh
  --cfg net="NETWORK" --cfg dex="DEX"
  ```

  where `NETWORK` and `DEX` should be replaced with the desired ones,
  while leaving the quotes.

  Example:

  ```sh
  --cfg net="main" --cfg dex="osmosis"
  ```

#### Workspaces

Th project is separated into three workspaces:

* `platform` - DEX-agnostic contracts and packages
* `protocol` - DEX-specific contracts and packages
* `tests` - integration tests

In the instructions below this value is stored in *WORKSPACE_DIR_NAME*.

#### A non-optimized version

The command below builds a contract if ran from the contract directory,
or builds the contracts of the workspace from within which it is ran:

```sh
RELEASE_VERSION='dev-release' RUSTFLAGS='--cfg net="dev" --cfg dex="osmosis"' cargo build --target=wasm32-unknown-unknown
```

One way to avoid having to set those environment variables is to
set them in the editor/IDE's configuration.

An example one for VSCode/VSCodium, located at `.vscode/settings.json`, is shown here:

```json
{
  "rust-analyzer.cargo.extraEnv": {
    "RUSTFLAGS": "--cfg net=\"dev\" --cfg dex=\"osmosis\"",
    "RELEASE_VERSION": "local",
  },
  "terminal.integrated.env.linux": {
    "RUSTFLAGS": "--cfg net=\"dev\" --cfg dex=\"osmosis\"",
    "RELEASE_VERSION": "local",
  },
  "terminal.integrated.env.osx": {
    "RUSTFLAGS": "--cfg net=\"dev\" --cfg dex=\"osmosis\"",
    "RELEASE_VERSION": "local",
  },
  "terminal.integrated.env.windows": {
    "RUSTFLAGS": "--cfg net=\"dev\" --cfg dex=\"osmosis\"",
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
`WORKSPACE_DIR_NAME`) and on the DEX (indicated by `DEX`) and just as the development build uses `RUSTFLAGS`
to indicate desired target network and DEX:

```sh
export WORKSPACE_DIR_NAME="platform"/"protocol"
export DEX="osmosis"
export ARTIFACTS_SUBDIR="$([[ $WORKSPACE_DIR_NAME == 'protocol' ]] && echo $DEX || echo 'platform')"

```

```sh
docker run --rm -v "$(pwd)/platform/:/platform/" \
  -v "$(pwd)/protocol/:/protocol/" \
  -v "$(pwd)/${WORKSPACE_DIR_NAME}/:/code/" \
  -v "$(pwd)/artifacts/${ARTIFACTS_SUBDIR}/:/artifacts/" \
  --env "RELEASE_VERSION=`git describe`-`date -Iminute`" \
  --env "RUSTFLAGS=--cfg net=\"dev\" --cfg dex=\"${DEX}\"" \
  wasm-optimizer
```

**NOTE:** As one might set those environment variables in the settings
of their editor/IDE, those environment variables still must be provided
as arguments to the `docker run` command.
Exception to this should be the `platform` workspace, as it strives to
be agnostic to the targetted network and DEX.

**NOTE:** Builds are reproducable *as long as* all environment variables
passed to the container are the exact same. If it is desired to build
a verification copy of the contracts, one must set the `RELEASE_VERSION`
environment variable to the one used to build the original instead.

### Test

Run the following in a package directory or any workspace.

```sh
cargo test --all-targets
```

As with the development build, workspaces may require setting `RUSTFLAGS`
in the same manner as with the development builds.

### Lint

Run the following in the all workspaces.

```sh
./lint.sh
```

As with the development build, workspaces may require setting `RUSTFLAGS`
in the same manner as with the development builds.

### New contracts

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

### Reordering contracts because one is now a dependency

As mentioned in the section above, contract addresses are dependent on the order
in which they are deployed in the script.

When changing the order of deployment, reorder the contracts' addresses accordingly,
so the order of the actual addresses is **not** changed but the contract who owns
that address is.

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
