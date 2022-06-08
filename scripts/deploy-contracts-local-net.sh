#!/bin/bash
set -euxo pipefail

COMMON_DIR="$(pwd)/scripts/common"
source "$COMMON_DIR"/cmd.sh

NOLUS_LOCAL_NET="http://localhost:26612"
CONTRACTS_RESULTS_FILE="contracts-info.json"
STABLE_DENOM="ibc/8A34AF0C1943FD0DFCDE9ADBF0B2C9959C45E87E6088EA2FC6ADACD59261B8A2"
HOME_DIR="$HOME/.nolus"

while [[ $# -gt 0 ]]; do
  key="$1"

  case $key in

  -h | --help)
    printf \
    "Usage: %s
    [--nolus-local-network <nolus-local-net-url>]
    [--contracts-result-json-file <json_file_name>]
    [--stable-denom <string>]
    [--home-dir <nolus_accounts_dir>]" \
     "$0"
    exit 0
    ;;

    --nolus-local-network)
    NOLUS_LOCAL_NET="$2"
    shift
    shift
    ;;

  --contract-result-json-file)
    CONTRACTS_RESULTS_FILE="$2"
    shift
    shift
    ;;

    --stable-denom)
    STABLE_DENOM="$2"
    shift
    shift
    ;;

    --home-dir)
    HOME_DIR="$2"
    shift
    shift
    ;;

  esac
done

# Deploy contracts

source "$COMMON_DIR"/deploy-contracts.sh
deployContracts "$CONTRACTS_RESULTS_FILE" "$NOLUS_LOCAL_NET" "$HOME_DIR" "$STABLE_DENOM"
