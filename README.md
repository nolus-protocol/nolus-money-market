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

### Build

* In debug:
```
cargo build
```

* In release:

```
cargo build --release
```
* TBD add WASM optimization, and introduce [just](https://github.com/casey/just)?

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


# References
[Node to Rust series](https://vino.dev/blog/node-to-rust-day-1-rustup/)