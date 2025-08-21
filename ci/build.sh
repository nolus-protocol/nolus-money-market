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
## busybox [with:]                                                            ##
##   * sha256sum                                                              ##
## [in-tree] cargo-each                                                       ##
## cargo [with:]                                                              ##
##   * Rust compiler                                                          ##
##   * Rust compiler [target=wasm32-unknown-unknown]                          ##
## jq                                                                         ##
################################################################################

set -eu

readonly BINARYEN_VERSION
: "${BINARYEN_VERSION:?}"

readonly CONTRACTS_COUNT
: "${CONTRACTS_COUNT:?}"

readonly COSMWASM_CAPABILITIES
: "${COSMWASM_CAPABILITIES?}"

readonly PRODUCTION_NETWORK_BUILD_PROFILE
: "${PRODUCTION_NETWORK_BUILD_PROFILE:?}"

readonly PRODUCTION_NETWORK_BUILD_PROFILE_DIRECTORY
: "${PRODUCTION_NETWORK_BUILD_PROFILE_DIRECTORY:?}"

readonly PRODUCTION_NETWORK_MAX_BINARY_SIZE
: "${PRODUCTION_NETWORK_MAX_BINARY_SIZE:?}"

readonly RUST_VERSION
: "${RUST_VERSION:?}"

readonly SOFTWARE_RELEASE_ID
: "${SOFTWARE_RELEASE_ID:?}"

readonly TEST_NETWORK_BUILD_PROFILE
: "${TEST_NETWORK_BUILD_PROFILE:?}"

readonly TEST_NETWORK_BUILD_PROFILE_DIRECTORY
: "${TEST_NETWORK_BUILD_PROFILE_DIRECTORY:?}"

readonly TEST_NETWORK_MAX_BINARY_SIZE
: "${TEST_NETWORK_MAX_BINARY_SIZE:?}"

readonly PROTOCOL_NETWORK
: "${PROTOCOL_NETWORK-""}"

readonly PROTOCOL_NAME
: "${PROTOCOL_NAME-""}"

case "${#}" in
  ("1")
    network_group="${1:?}"

    shift
    ;;
  (*)
    "echo" \
      "This script takes exactly one argument, the network group name!" \
      >&2

    exit "1"
esac

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

readonly profile
: "${profile:?}"

readonly max_binary_size
: "${max_binary_size:?}"

unoptimized_binaries_directory="${CARGO_TARGET_DIR:?}/wasm32-unknown-unknown/${output_directory:?}"
unset "output_directory"
readonly unoptimized_binaries_directory

optimized_binaries_directory="/artifacts/"
readonly optimized_binaries_directory

(
  artifacts_contents="$(
    "ls" \
      -A \
      "${optimized_binaries_directory:?}"
  )"

  case "${artifacts_contents?}" in
    ("") ;;
    (*)
      "echo" \
        "Artifacts directory is not empty!" \
        >&2

      exit "1"
  esac
)

generate_protocol_release_id() (
  files="$(
    "find" \
      "/src/build-configuration" \
      -type "f"
  )"

  hashes="$(
    while read -r "file"
    do
      case "${file}" in
        ("") continue ;;
      esac

      "sha256sum" "${file:?}"
    done \
      <<EOF
${files?}
EOF
  )"

  hash="$(
    "sha256sum" \
      <<EOF
${hashes?}
EOF
  )"

  "echo" "${hash%% *}"
)

___build_unoptimized() {
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
    --enable-bulk-memory-opt \
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

___calculate_optimized_binary_checksum() (
  case "${#}" in
    ("1")
      binary_name="${1:?}"

      shift
      ;;
    (*)
      "echo" \
        "\"___calculate_optimized_binary_checksum\" takes exactly one argument, the optimized binary's name!" \
        >&2

      exit "1"
  esac

  "echo" \
    "Calculating checksum for \"${binary_name:?}\"." \
    >&2

  "sha256sum" \
    "${optimized_binaries_directory}/${binary_name:?}" \
    >"${optimized_binaries_directory}/${binary_name:?}.sha256"
)

build() (
  case "${#}" in
    ("1")
      dex_type="${1:?}"
      shift
      ;;
    (*)
      "echo" \
        "The \"build\" function takes exactly one argument, the DEX type tag!" \
        >&2

      exit "1"
  esac

  "___build_unoptimized"

  unoptimized_binaries="$("___list_unoptimized_binaries")"
  readonly unoptimized_binaries

  if ! test -e "${optimized_binaries_directory:?}"
  then
    "mkdir" "${optimized_binaries_directory:?}"
  fi

  while read -r "file"
  do
    case "${file?}" in
      ("") continue ;;
    esac

    name="$("basename" "${file:?}")"

    "___optimize_binary" "${name:?}"

    "___check_optimized_binary" "${name:?}"

    "___calculate_optimized_binary_checksum" "${name:?}"
  done \
    <<EOF
${unoptimized_binaries:?}
EOF
)

get_dex_type() {
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
    <"/src/build-configuration/topology.json"
}

check_optimized_binaries_count() (
  cd "${optimized_binaries_directory:?}"

  files="$(
    "find" \
      "." \
      -type "f" \
      -name "?*.wasm"
  )"

  files_count="$(
    "wc" \
      -l \
      <<EOF
${files?}
EOF
)"

  case "${files_count:?}" in
    ("${CONTRACTS_COUNT:?}") ;;
    (*)
      files="$(
        while read -r "file"
        do
          case "${file?}" in
            ("") continue ;;
          esac

          "basename" "${file:?}"
        done \
          <<EOF
${files?}
EOF
      )"

      "echo" \
        "Expected ${CONTRACTS_COUNT:?} file(s), got ${files_count:?}!
Files:
${files?}" \
        >&2

      exit "1"
  esac
)

check_optimized_binaries_sizes() (
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

generate_informational_files() {
  "echo" \
    "${BINARYEN_VERSION:?}" \
    >"${optimized_binaries_directory:?}/binaryen-version.txt"

  "echo" \
    "${RUST_VERSION:?}" \
    >"${optimized_binaries_directory:?}/rust-version.txt"

  "echo" \
    "${SOFTWARE_RELEASE_ID:?}" \
    >"${optimized_binaries_directory:?}/software-release-id.txt"
}

copy_scripts_directory() {
  "cp" \
    -R \
    "/src/scripts" \
    "${optimized_binaries_directory:?}/"
}

workspace="$(
  path="$("pwd")"

  "basename" "${path:?}"
)"
readonly workspace

case "${workspace:?}" in
  ("platform") ;;
  ("protocol")
    BUILD_OUT_DIR_PATHS="/tmp/build-out-dir-paths"
    readonly BUILD_OUT_DIR_PATHS
    export BUILD_OUT_DIR_PATHS

    PROTOCOL_RELEASE_ID="$("generate_protocol_release_id")"
    readonly PROTOCOL_RELEASE_ID
    export PROTOCOL_RELEASE_ID

    "echo" \
      "${PROTOCOL_RELEASE_ID:?}" \
      >"${optimized_binaries_directory:?}/protocol-release-id.txt"
    ;;
  (*)
    "echo" \
      "Unknown workspace!" \
      >&2

    exit "1"
esac

"build" "@agnostic"

case "${workspace:?}" in
  ("protocol")
    outputs_directory="${optimized_binaries_directory:?}/outputs"
    readonly outputs_directory

    "mkdir" "${outputs_directory:?}"

    dex_type="$("get_dex_type")"

    "build" "dex-${dex_type:?}"

    directories="$(
      directories="$("cat" "${BUILD_OUT_DIR_PATHS:?}")"

      case "${directories?}" in
        ("") exit ;;
      esac

      "sort" \
        -u \
        <<EOF
${directories:?}
EOF
    )"

    while read -r "directory"
    do
      case "${directory?}" in
        ("") continue ;;
      esac

      out_dir_name="$(
        build_dir_path="$("dirname" "${directory:?}")"

        "basename" "${build_dir_path:?}"
      )"

      "mv" \
        "${directory:?}" \
        "${outputs_directory:?}/${out_dir_name:?}"
    done \
      <<EOF
${directories:?}
EOF
    ;;
esac

"check_optimized_binaries_count"

"check_optimized_binaries_sizes"

"generate_informational_files"

"copy_scripts_directory"
