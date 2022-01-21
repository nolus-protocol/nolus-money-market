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
