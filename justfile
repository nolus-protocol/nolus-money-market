alias b := build
alias fmt := format

export RUSTFLAGS := ""
export RELEASE_VERSION := "local"
export NET_NAME := "dev"

full: fix lint build test check-wasm

fix:
    cargo clippy --fix --allow-dirty --allow-staged --workspace --verbose --all-targets -- -D warnings -D future-incompatible -D nonstandard-style -D rust-2018-compatibility -D rust-2018-idioms -D rust-2021-compatibility -D unused -D clippy::all
    @just format

format:
    @cargo fmt

check: format
    cargo hack check

lint: format
    ./lint.sh

build:
    cargo build

test:
    cargo test
    cargo test --release

admin_schema:
    cargo run --example admin_schema -- .

schema package:
    cargo run -p {{package}} --example schema -- .

doc:
    cargo doc --open

check-wasm:
    cargo build --target wasm32-unknown-unknown
    cosmwasm-check --available-capabilities staking,stargate,cosmwasm_1_1,iterator,neutron ./target/wasm32-unknown-unknown/debug/*.wasm
