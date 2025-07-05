{
  lib,
  profile,
  rust-stable,
  stdenv,
  writeShellScriptBin,
}:
let
  path = lib.makeBinPath [
    rust-stable
    stdenv.cc
  ];
in
writeShellScriptBin "run-tests" ''
  set -eu

  profile="$("${profile}/bin/profile")"
  readonly "profile"

  PATH="${path}" \
    "cargo" \
    "test" \
    --all-targets \
    --no-default-features \
    --profile "''${profile}" \
    "''${@}"
''
