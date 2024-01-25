#!/bin/bash
set -euxo pipefail

# Example, deploy new contracts DEX=osmosis:
#
# ./scripts/deploy-contracts-live.sh deploy_contracts "http://localhost:26612/" "nolus-local-v0.4.1-21-1700055474" \
# "$HOME/.nolus" "wasmAdmin" "storeCodePrivilegedUser" "nolus1gurgpv8savnfw66lckwzn4zk7fp394lpe667dhu7aw48u40lj6jsqxf8nd" \
# "$HOME/Documents/nolus/nolus-money-market/artifacts/osmosis" "OSMOSIS" "OSMOSIS" \
# "connection-0" "channel-0" "channel-2048" \
# "USDC" "nolus14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9s0k0puz" \
# "nolus1zwv6feuzhy6a9wekh96cd57lsarmqlwxdypdsplw6zhfncqw6ftqmx7chl"  \
# '{"value":[0,"USDC"],"children":[{"value":[5,"OSMO"],"children":[{"value":[12,"ATOM"]}]}]}'

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
source "$SCRIPT_DIR"/common/cmd.sh

_wait_tx_included_in_block() {
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

_get_predictable_contract_address() {
  local -r nolus_net="$1"
  local -r nolus_home_dir="$2"
  local -r admin_contract_address="$3"
  local -r code_id="$4"
  local -r protocol="$5"

  local -r get_predictable_contract_address_msg='{"instantiate_address":{"code_id":'"$code_id"',"protocol":"'"$protocol"'"}}'
  local -r expected_address=$(run_cmd "$nolus_home_dir" q wasm contract-state smart "$admin_contract_address" "$get_predictable_contract_address_msg" --node "$nolus_net" --output json | jq '.data'  | tr -d '"')

  echo "$expected_address"
}

_store_code() {
  local -r nolus_net="$1"
  local -r chain_id="$2"
  local -r nolus_home_dir="$3"
  local -r store_code_privileged_wallet_key="$4"
  local -r wasm_code_path="$5"
  local -r instantiate_only_address="$6"

  local store_result
  store_result=$(run_cmd "$nolus_home_dir" tx wasm store "$wasm_code_path" --instantiate-anyof-addresses "$instantiate_only_address" --from "$store_code_privileged_wallet_key" $FLAGS --yes --output json)
  store_result=$(echo "$store_result" | sed -n '/{/{p; q}')
  local -r store_tx_hash=$(echo "$store_result" | jq -r '.txhash')

  local -r store_tx=$(_wait_tx_included_in_block "$nolus_home_dir" "$nolus_net" "$store_tx_hash")
  local -r code_id=$(echo "$store_tx" | jq -r '.logs[].events[] | select(.type == "store_code") | .attributes[] | select(.key == "code_id") | .value')
  echo "$code_id"
}

_instantiate() {
  local -r nolus_net="$1"
  local -r chain_id="$2"
  local -r nolus_home_dir="$3"
  local -r dex_admin_wallet_key="$4"
  local -r code_id="$5"
  local -r init_msg="$6"
  local -r label="$7"
  local -r protocol="$8"
  local -r expected_address="$9"
  local -r admin_contract_address="${10}"

  local -r escaped_string="${init_msg//\"/\\\"}"

  local -r instantiate_new_contract_exec_msg='{"instantiate":{"code_id":'"$code_id"',"label":"'"$label"'","message":"'"$escaped_string"'","protocol":"'"$protocol"'","expected_address":"'"$expected_address"'"}}'
  local instantiate_result
  instantiate_result=$(run_cmd "$nolus_home_dir" tx wasm execute "$admin_contract_address" "$instantiate_new_contract_exec_msg" --from "$dex_admin_wallet_key" $FLAGS --yes --output json)
  instantiate_result=$(echo "$instantiate_result" | sed -n '/{/{p; q}')
  local -r instantiate_tx_hash=$(echo "$instantiate_result" | jq -r '.txhash')

  local -r instantiate_tx=$(_wait_tx_included_in_block "$nolus_home_dir" "$nolus_net" "$instantiate_tx_hash")
  local -r contract_address=$(echo "$instantiate_tx" |  jq -r '.logs[].events[] | select(.type == "instantiate") | .attributes[] | select(.key == "_contract_address") | .value')

  echo "$contract_address"
}

_deploy_contract() {
  set -euo pipefail
  local -r nolus_net="$1"
  local -r chain_id="$2"
  local -r nolus_home_dir="$3"
  local -r dex_admin_wallet_key="$4"
  local -r store_code_privileged_wallet_key="$5"
  local -r admin_contract_address="$6"
  local -r wasm_code_path="$7"
  local -r init_msg="$8"
  local -r label="$9"
  local -r protocol="${10}"

  local -r code_id=$(_store_code "$nolus_net" "$chain_id" "$nolus_home_dir" "$store_code_privileged_wallet_key" "$wasm_code_path" "$admin_contract_address")

  local -r expected_address=$(_get_predictable_contract_address "$nolus_net" "$nolus_home_dir" "$admin_contract_address" "$code_id" "$protocol")
  local -r contract_address=$(_instantiate "$nolus_net" "$chain_id" "$nolus_home_dir" "$dex_admin_wallet_key" "$code_id" "$init_msg" "$label" "$protocol" "$expected_address" "$admin_contract_address")

  echo "$contract_address"
}

deploy_contracts() {
  set -euo pipefail
  local -r nolus_net="$1"
  local -r chain_id="$2"
  local -r nolus_home_dir="$3"
  local -r dex_admin_wallet_key="$4"
  local -r store_code_privileged_wallet_key="$5"
  local -r admin_contract_address="$6"
  local -r wasm_path="$7"
  local -r network="$8"
  local -r dex="$9"
  local -r dex_connection="${10}"
  local -r dex_channel_local="${11}"
  local -r dex_channel_remote="${12}"
  local -r protocol_currency="${13}"
  local -r treasury_contract_address="${14}"
  local -r timealarms_contract_address="${15}"
  local swap_tree="${16}"
  swap_tree=$(echo "$swap_tree" | sed 's/^"\(.*\)"$/\1/')

  local protocol="${network}-${dex}-${protocol_currency}"
  protocol="${protocol^^}"

  FLAGS="--broadcast-mode sync --fees 200000unls --gas auto --gas-adjustment 1.2 --node $nolus_net --chain-id $chain_id"

  # upload Leaser code
  local -r leaser_code_id=$(_store_code "$nolus_net" "$chain_id" "$nolus_home_dir" "$store_code_privileged_wallet_key" "$wasm_path/leaser.wasm"  "$admin_contract_address")

  # upload Lease code
  local -r leaser_expected_address=$(_get_predictable_contract_address "$nolus_net" "$nolus_home_dir" "$admin_contract_address" "$leaser_code_id" "$protocol")
  local -r lease_code_id=$(_store_code "$nolus_net" "$chain_id" "$nolus_home_dir" "$store_code_privileged_wallet_key" "$wasm_path/lease.wasm"  "$leaser_expected_address")

  # upload and instantiate LPP
  local -r lpp_init_msg='{"lpn_ticker":"'"$protocol_currency"'","lease_code_admin":"'"$leaser_expected_address"'","borrow_rate":{"base_interest_rate":100,"utilization_optimal":750,"addon_optimal_interest_rate":20},"min_utilization":0}'
  local -r lpp_contract_address=$(_deploy_contract "$nolus_net" "$chain_id" "$nolus_home_dir"  "$dex_admin_wallet_key" "$store_code_privileged_wallet_key" "$admin_contract_address" "$wasm_path/lpp.wasm" "$lpp_init_msg" "$protocol-lpp" "$protocol")

  # upload and instantiate Oracle
  local -r oracle_init_msg='{"config":{"base_asset":"'"$protocol_currency"'","price_config":{"min_feeders":500,"sample_period_secs":10,"samples_number":12,"discount_factor":750}},"swap_tree":'"$swap_tree"'}'
  local -r oracle_contract_address=$(_deploy_contract "$nolus_net" "$chain_id" "$nolus_home_dir" "$dex_admin_wallet_key" "$store_code_privileged_wallet_key" "$admin_contract_address" "$wasm_path/oracle.wasm" "$oracle_init_msg" "$protocol-oracle" "$protocol")

  # upload and instantiate Profit
  local -r profit_init_msg='{"cadence_hours":7200,"treasury":"'"$treasury_contract_address"'","oracle":"'"$oracle_contract_address"'","timealarms":"'"$timealarms_contract_address"'","dex":{"connection_id":"'"$dex_connection"'","transfer_channel":{"local_endpoint":"'"$dex_channel_local"'","remote_endpoint":"'"$dex_channel_remote"'"}}}'
  local -r profit_contract_address=$(_deploy_contract "$nolus_net" "$chain_id" "$nolus_home_dir" "$dex_admin_wallet_key" "$store_code_privileged_wallet_key" "$admin_contract_address" "$wasm_path/profit.wasm" "$profit_init_msg" "$protocol-profit" "$protocol")

  # instantiate Leaser
  local -r leaser_init_msg='{"lease_code_id":"'"$lease_code_id"'","lease_interest_rate_margin":30,"lease_position_spec":{"liability":{"initial":650,"healthy":700,"first_liq_warn":720,"second_liq_warn":750,"third_liq_warn":780,"max":800,"recalc_time":7200000000000},"min_asset":{"amount":"150","ticker":"'"$protocol_currency"'"},"min_transaction":{"amount":"10","ticker":"'"$protocol_currency"'"}},"lpp_ust_addr":"'"$lpp_contract_address"'","time_alarms":"'"$timealarms_contract_address"'","market_price_oracle":"'"$oracle_contract_address"'","profit":"'"$profit_contract_address"'","lease_interest_payment":{"due_period":5184000000000000,"grace_period":864000000000000},"dex":{"connection_id":"'"$dex_connection"'","transfer_channel":{"local_endpoint":"'"$dex_channel_local"'","remote_endpoint":"'"$dex_channel_remote"'"}}}'
  local -r leaser_contract_address=$(_instantiate "$nolus_net" "$chain_id" "$nolus_home_dir" "$dex_admin_wallet_key" "$leaser_code_id" "$leaser_init_msg" "$protocol-leaser" "$protocol" "$leaser_expected_address" "$admin_contract_address" )

  # register the protocol
  local -r add_protocol_set_exec_msg='{"register_protocol":{"name":"'"$protocol"'","protocol":{"network":"'"$network"'","contracts":{"leaser":"'"$leaser_contract_address"'","lpp":"'"$lpp_contract_address"'","oracle":"'"$oracle_contract_address"'","profit":"'"$profit_contract_address"'"}}}}'
  run_cmd "$nolus_home_dir" tx wasm execute "$admin_contract_address" "$add_protocol_set_exec_msg" --from "$dex_admin_wallet_key" $FLAGS --yes
}

if [ "$#" -ne 0 ]; then
  case "$1" in
    deploy_contracts)
      deploy_contracts "$2" "$3" "$4" "$5" "$6" "$7" "$8" "$9" "${10}" "${11}" "${12}" "${13}" "${14}" "${15}" "${16}"
      ;;
    *)
      echo "Unknown function: $1"
      exit 1
      ;;
  esac
fi
