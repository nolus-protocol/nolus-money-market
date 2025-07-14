{
  cargo-udeps,
  lib,
  rust-nightly,
  stdenv,
  writeShellScriptBin,
}:
let
  path = lib.makeBinPath [
    cargo-udeps
    rust-nightly
    stdenv.cc
  ];
in
writeShellScriptBin "check-unused-deps" ''
  PATH="${path}" \
    "cargo" \
    "udeps" \
    --all-targets \
    --no-default-features \
    "''${@}"
''
