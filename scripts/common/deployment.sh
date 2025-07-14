#!/bin/bash

CURRENT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
source "$CURRENT_DIR"/cmd.sh
source "$CURRENT_DIR"/tx.sh

store_code() {
  local -r nolus_net="$1"
  local -r nolus_home_dir="$2"
  local -r store_code_privileged_wallet_key="$3"
  local -r wasm_code_path="$4"
  local -r instantiate_policy="$5"

  local -r store_result=$(run_cmd "$nolus_home_dir" tx wasm store "$wasm_code_path" $instantiate_policy --from "$store_code_privileged_wallet_key" $FLAGS --yes --output json)
  local -r store_tx_hash=$(get_tx_hash "$store_result")

  local -r store_tx=$(wait_tx_included_in_block "$nolus_home_dir" "$nolus_net" "$store_tx_hash")
  local -r code_id=$(echo "$store_tx" | jq -r '.events[] | select(.type == "store_code") | .attributes[] | select(.key == "code_id") | .value')

  echo "$code_id"
}