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
##   * rustc                                                                  ##
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
shift

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

  unoptimized_binaries="$(
    cd "${unoptimized_binaries_directory:?}"

    "find" \
      "." \
      "(" \
      "!" \
      -path "./?*/?*" \
      ")" \
      -type "f" \
      -name "*.wasm"
  )"
  readonly "unoptimized_binaries"

  if ! test -e "${optimized_binaries_directory:?}"
  then
    "mkdir" "${optimized_binaries_directory:?}"
  fi

  cd "${unoptimized_binaries_directory:?}"

  (
    files_count="$(
      "wc" \
        -l \
        <<EOF
${unoptimized_binaries?}
EOF
  )"

    case "${files_count:?}" in
      ("${CONTRACTS_COUNT:?}") ;;
      (*)
        "echo" \
          "Expected ${CONTRACTS_COUNT:?} file(s), got ${files_count:?}!
Files:
${unoptimized_binaries?}" \
          >&2

        exit "1"
    esac
  )

  while read -r "file"
  do
    case "${file?}" in
      ("") continue ;;
    esac

    name="$("basename" "${file:?}")"

    "echo" \
      "Optimizing \"${name:?}\"." \
      >&2

    "wasm-opt" \
      --inlining-optimizing \
      -Os \
      -o "${optimized_binaries_directory}/${name:?}" \
      --signext-lowering \
      "${file:?}"

    "echo" \
      "Checking \"${name:?}\"." \
      >&2

    "/usr/local/bin/cosmwasm-check" \
      --available-capabilities "${COSMWASM_CAPABILITIES:?}" \
      "${optimized_binaries_directory}/${name:?}"
  done <<EOF
${unoptimized_binaries:?}
EOF

  cd "${optimized_binaries_directory}"
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
