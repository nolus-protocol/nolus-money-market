#!/bin/bash

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"/cmd.sh

wait_tx_included_in_block() {
  local -r nolus_home_dir="$1"
  local -r nolus_net="$2"
  local -r tx_hash="$3"

  local tx_state="NOT_INCLUDED"

  while [ "$tx_state" == "NOT_INCLUDED"  ]
  do
    sleep 1
    tx_state=$(run_cmd "$nolus_home_dir" q tx "$tx_hash" --node "$nolus_net" --output json) || tx_state="NOT_INCLUDED"
  done

  echo "$tx_state"
}

get_tx_hash() {
  local -r tx_result="$1"

  local -r tx_result_json=$(echo "$tx_result" | sed -n '/{/{p; q}')
  local -r tx_hash=$(echo "$tx_result_json" | jq -r '.txhash')

  echo "$tx_hash"

}
