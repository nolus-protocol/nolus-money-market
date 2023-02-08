#!/bin/sh

# -D warnings is set so we allow 'deprecated' lints to tolerate them
# remove '-A deprecated' to find out if we use any
cargo clippy --workspace --verbose --all-targets -- -D warnings -D future-incompatible \
  -D nonstandard-style -D rust-2018-compatibility -D rust-2018-idioms \
  -D rust-2021-compatibility -D unused -D clippy::all
