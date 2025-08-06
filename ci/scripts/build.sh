#!/usr/bin/env sh

################################################################################
## This script shall conform to the POSIX.1 standard, a.k.a. IEEE Std 1003.1. ##
## When utilities defined in the standard are to be invoked, they shall only  ##
## be invoked utilizing functions defined by the standard, excluding any and  ##
## all extensions to the standard functions, e.g. GNU extensions.             ##
##                                                                            ##
## Version of the POSIX.1 standard used: POSIX.1-2008                         ##
## https://pubs.opengroup.org/onlinepubs/9699919799.2008edition/              ##
##                                                                            ##
## Used version of the standard should not be moved forward unless necessary  ##
## in order to keep the script as portable as possible between different      ##
## environments.                                                              ##
##                                                                            ##
## Used version of the standard should be moved backwards if possible in      ##
## order to keep the script as portable as possible between different         ##
## environments.                                                              ##
################################################################################
## Used utilities outside the POSIX standard:                                 ##
## [in-tree] cargo-each                                                       ##
## cargo [with:]                                                              ##
##   * Rust compiler                                                          ##
##   * Rust compiler [target=wasm32-unknown-unknown]                          ##
## jq                                                                         ##
################################################################################

set -eu

RUSTFLAGS="${RUSTFLAGS:+"${RUSTFLAGS} "}-C link-arg=-s"
readonly "RUSTFLAGS"
export "RUSTFLAGS"

readonly "CONTRACTS_COUNT"
: "${CONTRACTS_COUNT:?}"
export "CONTRACTS_COUNT"

readonly "COSMWASM_CAPABILITIES"
: "${COSMWASM_CAPABILITIES?}"
export "COSMWASM_CAPABILITIES"

readonly "PRODUCTION_NETWORK_BUILD_PROFILE"
: "${PRODUCTION_NETWORK_BUILD_PROFILE:?}"
export "PRODUCTION_NETWORK_BUILD_PROFILE"

readonly "PRODUCTION_NETWORK_BUILD_PROFILE_DIRECTORY"
: "${PRODUCTION_NETWORK_BUILD_PROFILE_DIRECTORY:?}"
export "PRODUCTION_NETWORK_BUILD_PROFILE_DIRECTORY"

readonly "PRODUCTION_NETWORK_MAX_BINARY_SIZE"
: "${PRODUCTION_NETWORK_MAX_BINARY_SIZE:?}"
export "PRODUCTION_NETWORK_MAX_BINARY_SIZE"

readonly "TEST_NETWORK_BUILD_PROFILE"
: "${TEST_NETWORK_BUILD_PROFILE:?}"
export "TEST_NETWORK_BUILD_PROFILE"

readonly "TEST_NETWORK_BUILD_PROFILE_DIRECTORY"
: "${TEST_NETWORK_BUILD_PROFILE_DIRECTORY:?}"
export "TEST_NETWORK_BUILD_PROFILE_DIRECTORY"

readonly "TEST_NETWORK_MAX_BINARY_SIZE"
: "${TEST_NETWORK_MAX_BINARY_SIZE:?}"
export "TEST_NETWORK_MAX_BINARY_SIZE"

readonly "PROTOCOL_NETWORK"

readonly "PROTOCOL_NAME"

readonly "PROTOCOL_RELEASE_ID"

case "${#}" in
  ("1") ;;
  (*)
    "echo" \
      "This script takes exactly one argument, the network group name!" \
      >&2

    exit "1"
esac

network_group="${1:?}"
shift

case "${network_group:?}" in
  ("test-net")
    profile="${TEST_NETWORK_BUILD_PROFILE:?}"

    output_directory="${TEST_NETWORK_BUILD_PROFILE_DIRECTORY:?}"

    max_binary_size="${TEST_NETWORK_MAX_BINARY_SIZE:?}"
    ;;
  ("production-net")
    profile="${PRODUCTION_NETWORK_BUILD_PROFILE:?}"

    output_directory="${PRODUCTION_NETWORK_BUILD_PROFILE_DIRECTORY:?}"

    max_binary_size="${PRODUCTION_NETWORK_MAX_BINARY_SIZE:?}"
    ;;
  (*)
    "echo" \
      "Unknown network group!" \
      >&2
esac

readonly "profile"
: "${profile:?}"

readonly "max_binary_size"
: "${max_binary_size:?}"

unoptimized_binaries_directory="${CARGO_TARGET_DIR:?}/wasm32-unknown-unknown/${output_directory:?}"
unset "output_directory"
readonly "unoptimized_binaries_directory"

optimized_binaries_directory="/artifacts/"
readonly optimized_binaries_directory

(
  artifacts_contents="$("ls" -A "${optimized_binaries_directory:?}")"

  case "${artifacts_contents?}" in
    ("") ;;
    (*)
      "echo" \
        "Artifacts directory is not empty!" \
        >&2

      exit "1"
  esac
)

___build_unoptimized() {
  case "${#}" in
    ("0") ;;
    (*)
      "echo" \
        "\"___build_unoptimized\" takes no arguments!" \
        >&2

      exit "1"
  esac

  "cargo" \
    "each" \
    --tag "build" \
    --tag "${dex_type:?}" \
    "run" \
    --exact \
    -- \
    "build" \
    --profile "${profile:?}" \
    --lib \
    --locked \
    --target "wasm32-unknown-unknown"
}

___list_unoptimized_binaries() (
  case "${#}" in
    ("0") ;;
    (*)
      "echo" \
        "\"___list_unoptimized_binaries\" takes no arguments!" \
        >&2

      exit "1"
  esac

  cd "${unoptimized_binaries_directory:?}"

  "find" \
    "." \
    "(" \
    "!" \
    -path "./?*/?*" \
    ")" \
    -type "f" \
    -name "*.wasm"
)

___check_binaries_count() (
  case "${#}" in
    ("1")
      binaries_list="${1?}"

      shift
      ;;
    (*)
      "echo" \
        "\"___check_binaries_count\" takes exactly one argument, the binaries list!" \
        >&2

      exit "1"
  esac

  cd "${unoptimized_binaries_directory:?}"

  files_count="$(
    "wc" \
      -l \
      <<EOF
${binaries_list?}
EOF
)"

  case "${files_count:?}" in
    ("${CONTRACTS_COUNT:?}") ;;
    (*)
      "echo" \
        "Expected ${CONTRACTS_COUNT:?} file(s), got ${files_count:?}!
Files:
${binaries_list?}" \
        >&2

      exit "1"
  esac
)

___optimize_binary() (
  case "${#}" in
    ("1")
      binary_name="${1:?}"

      shift "1"
      ;;
    (*)
      "echo" \
        "\"___optimize_binary\" takes exactly one argument, the binary's file name!" \
        >&2

      exit "1"
  esac

  "echo" \
    "Optimizing \"${binary_name:?}\"." \
    >&2

  "wasm-opt" \
    --inlining-optimizing \
    -Os \
    -o "${optimized_binaries_directory}/${binary_name:?}" \
    --signext-lowering \
    "${unoptimized_binaries_directory:?}/${binary_name:?}"
)

___check_optimized_binary() (
  case "${#}" in
    ("1")
      binary_name="${1:?}"

      shift
      ;;
    (*)
      "echo" \
        "\"___check_optimized_binary\" takes exactly one argument, the optimized binary's name!" \
        >&2

      exit "1"
  esac

  "echo" \
    "Checking \"${binary_name:?}\"." \
    >&2

  "cosmwasm-check" \
    --available-capabilities "${COSMWASM_CAPABILITIES:?}" \
    "${optimized_binaries_directory}/${binary_name:?}"
)

___check_optimized_binaries_sizes() (
  case "${#}" in
    ("0") ;;
    (*)
      "echo" \
        "\"___check_optimized_binaries_sizes\" takes no arguments!" \
        >&2

      exit "1"
  esac

  cd "${optimized_binaries_directory:?}"

  large_files="$(
    "find" \
      "." \
      -type "f" \
      -name "*.wasm" \
      -size "+${max_binary_size:?}c" \
      -print
  )"

  case "${large_files?}" in
    ("") return ;;
  esac

  large_files="$(
    while read -r "file"
    do
      stats="$(
        "ls" \
          -ks \
          -1 \
          "${file:?}"
      )"

      awk \
        "{ print \$2 - \$1 KiB }" \
        <<EOF
${stats:?}
EOF
    done \
      <<EOF
${large_files:?}
EOF
  )"

  "echo" \
    "Some files are over the allowed limit of ${max_binary_size:?} byte(s):
${large_files:?}" \
    >&2

  exit "1"
)

build() (
  dex_type="${1:?}"
  shift

  case "${#}" in
    ("0") ;;
    (*)
      "echo" \
        "The \"build\" function takes exactly one argument, the DEX type tag!" \
        >&2

      exit "1"
  esac

  "___build_unoptimized"

  unoptimized_binaries="$("___list_unoptimized_binaries")"
  readonly "unoptimized_binaries"

  if ! test -e "${optimized_binaries_directory:?}"
  then
    "mkdir" "${optimized_binaries_directory:?}"
  fi

  "___check_binaries_count" "${unoptimized_binaries?}"

  while read -r "file"
  do
    case "${file?}" in
      ("") continue ;;
    esac

    name="$("basename" "${file:?}")"

    "___optimize_binary" "${name:?}"

    "___check_optimized_binary" "${name:?}"
  done \
    <<EOF
${unoptimized_binaries:?}
EOF

  "___check_optimized_binaries_sizes"
)

"build" "@agnostic"

workspace="$(
  path="$("pwd")"

  "basename" "${path:?}"
)"

case "${workspace:?}" in
  ("platform") ;;
  ("protocol")
    CURRENCIES_BUILD_REPORT="/artifacts/currencies.build.log"
    readonly "CURRENCIES_BUILD_REPORT"
    export "CURRENCIES_BUILD_REPORT"

    dex_type="$(
      protocol="$(
        "jq" \
          -c \
          "." \
          <"/src/build-configuration/protocol.json"
      )"

      "jq" \
        --argjson "protocol" "${protocol:?}" \
        --exit-status \
        --raw-output \
        ".networks[\$protocol.dex_network].dexes[\$protocol.dex].type | select(. != null)" \
        <"${topology_json:?}"
    )"

    "build" "${dex_type:?}"
    ;;
  (*)
    "echo" \
      "Unknown workspace!" \
      >&2

    exit "1"
esac
