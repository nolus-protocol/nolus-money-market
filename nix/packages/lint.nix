{
  lib,
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
writeShellScriptBin "lint" ''
  set -eu

  case "''${USE_PROFILE+"1"}" in
    ("1")
      profile="''${USE_PROFILE}"
      ;;
    (*)
      profile="''${1?"Profile not passed as \"USE_PROFILE\", nor as first parameter!"}"

      shift
  esac
  readonly "profile"

  PATH="${path}" \
    "cargo" \
    "clippy" \
    --all-targets \
    --no-default-features \
    --profile "''${profile}" \
    "''${@}" \
    -- \
    --forbid "deprecated-safe" \
    --deny "future-incompatible" \
    --deny "keyword-idents" \
    --deny "nonstandard-style" \
    --deny "refining-impl-trait" \
    --deny "rust-2018-idioms" \
    --deny "unused" \
    \
    --forbid "unfulfilled_lint_expectations" \
    \
    --deny "clippy::all" \
    --deny "clippy::unwrap_used" \
    --deny "clippy::unwrap_in_result" \
    \
    --deny "warnings" \
    \
    --allow "clippy::large_enum_variant"
''
