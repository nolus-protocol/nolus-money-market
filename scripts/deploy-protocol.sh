#!/bin/bash
set -euxo pipefail

# Example, deploy new contracts DEX=Osmosis:
#
# ./scripts/deploy-contracts-live.sh deploy_contracts "http://localhost:26612/" "nolus-local-v0.4.1-21-1700055474" \
# "$HOME/.nolus" "wasmAdmin" "storeCodePrivilegedUser" "<lease-admin-address>" "nolus17p9rzwnnfxcjp32un9ug7yhhzgtkhvl9jfksztgw5uh69wac2pgsmc5xhq" \
# "$HOME/Documents/nolus/nolus-money-market/artifacts/osmosis" "Osmosis" "Osmosis" \
# '"Osmosis"' "connection-0" "channel-0" "channel-2048" \
# "USDC" "nolus1nc5tatafv6eyq7llkr2gv50ff9e22mnf70qgjlv737ktmt4eswrqrr2r7y" \
# "nolus14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9s0k0puz"  \
# '{"value":[0,"USDC"],"children":[{"value":[5,"OSMO"],"children":[{"value":[12,"ATOM"]}]}]}'

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
source "$SCRIPT_DIR"/common/cmd.sh
source "$SCRIPT_DIR"/common/deployment.sh
source "$SCRIPT_DIR"/common/tx.sh

_key_get_address() {
  local -r nolus_home_dir="$1"
  local -r key="$2"

  local keyInfo
  if ! keyInfo=$(run_cmd "$nolus_home_dir" keys show "$key" --output json); then
    exit 1
  fi

  local -r address=$(echo "$keyInfo" | jq -r '.address')
  echo "$address"
}

_nls_balance_amount_check() {
  local -r nolus_home_dir="$1"
  local -r nolus_net="$2"
  local -r address="$3"

  run_cmd "$nolus_home_dir" q bank balances "$address" --output json --node "$nolus_net" | jq -r '.balances[] | select(.denom == "unls") | .amount'
}

_keys_check() {
  local -r nolus_net="$1"
  local -r nolus_home_dir="$2"
  local -r dex_admin_wallet_key="$3"
  local -r store_code_privileged_wallet_key="$4"

  local dex_admin_address
  local store_code_privileged_address

  if ! dex_admin_address=$(_key_get_address "$nolus_home_dir" "$dex_admin_wallet_key"); then
    echo "Error: Failed to get address for key $dex_admin_wallet_key. Verify that the key actually exists"
    exit 1
  fi

  if ! store_code_privileged_address=$(_key_get_address "$nolus_home_dir" "$store_code_privileged_wallet_key"); then
    echo "Error: Failed to get address for key $store_code_privileged_wallet_key. Verify that the key actually exists."
    exit 1
  fi

  local -r store_code_privileged_address_balance=$(_nls_balance_amount_check "$nolus_home_dir" "$nolus_net" "$dex_admin_address")
  local -r dex_admin_address_balance=$(_nls_balance_amount_check "$nolus_home_dir" "$nolus_net" "$store_code_privileged_address")

  local -r threshold=5000000
  local are_valid=true
  if [ "$(echo "$store_code_privileged_address_balance < $threshold" | bc)" -eq 1 ] || [ "$(echo "$dex_admin_address_balance < $threshold" | bc)" -eq 1 ]; then
  are_valid=false
    echo "W: Balance for denom 'unls' is less than $threshold in one of the wallets."
  fi

  echo "$are_valid"
}

_get_predictable_contract_address() {
  local -r nolus_net="$1"
  local -r nolus_home_dir="$2"
  local -r admin_contract_address="$3"
  local -r code_id="$4"
  local -r protocol="$5"

  local -r get_predictable_contract_address_msg='{"instantiate_address":{"code_id":"'"$code_id"'","protocol":"'"$protocol"'"}}'
  local -r expected_address=$(run_cmd "$nolus_home_dir" q wasm contract-state smart "$admin_contract_address" "$get_predictable_contract_address_msg" --node "$nolus_net" --output json | jq '.data'  | tr -d '"')

  echo "$expected_address"
}


_instantiate() {
  local -r nolus_net="$1"
  local -r nolus_home_dir="$2"
  local -r dex_admin_wallet_key="$3"
  local -r code_id="$4"
  local -r init_msg="$5"
  local -r label="$6"
  local -r protocol="$7"
  local -r expected_address="$8"
  local -r admin_contract_address="$9"

  local -r escaped_string="${init_msg//\"/\\\"}"

  local -r instantiate_new_contract_exec_msg='{"instantiate":{"code_id":"'"$code_id"'","label":"'"$label"'","message":"'"$escaped_string"'","protocol":"'"$protocol"'","expected_address":"'"$expected_address"'"}}'
  local instantiate_result
  instantiate_result=$(run_cmd "$nolus_home_dir" tx wasm execute "$admin_contract_address" "$instantiate_new_contract_exec_msg" --from "$dex_admin_wallet_key" $FLAGS --yes --output json)
  instantiate_result=$(echo "$instantiate_result" | sed -n '/{/{p; q}')
  local -r instantiate_tx_hash=$(echo "$instantiate_result" | jq -r '.txhash')

  local -r instantiate_tx=$(wait_tx_included_in_block "$nolus_home_dir" "$nolus_net" "$instantiate_tx_hash")
  local -r contract_address=$(echo "$instantiate_tx" |  jq -r '.events[] | select(.type == "instantiate") | .attributes[] | select(.key == "_contract_address") | .value')

  echo "$contract_address"
}

_deploy_contract() {
  set -euo pipefail
  local -r nolus_net="$1"
  local -r nolus_home_dir="$2"
  local -r dex_admin_wallet_key="$3"
  local -r store_code_privileged_wallet_key="$4"
  local -r admin_contract_address="$5"
  local -r wasm_code_path="$6"
  local -r init_msg="$7"
  local -r label="$8"
  local -r protocol="$9"

  local -r code_id=$(store_code "$nolus_net" "$nolus_home_dir" "$store_code_privileged_wallet_key" "$wasm_code_path" "--instantiate-anyof-addresses $admin_contract_address")

  local -r expected_address=$(_get_predictable_contract_address "$nolus_net" "$nolus_home_dir" "$admin_contract_address" "$code_id" "$protocol")
  local -r contract_address=$(_instantiate "$nolus_net" "$nolus_home_dir" "$dex_admin_wallet_key" "$code_id" "$init_msg" "$label" "$protocol" "$expected_address" "$admin_contract_address")

  echo "$contract_address"
}

deploy_contracts() {
  set -euo pipefail
  local -r nolus_net="$1"
  local -r chain_id="$2"
  local -r nolus_home_dir="$3"
  local -r dex_admin_wallet_key="$4"
  local -r store_code_privileged_wallet_key="$5"
  local -r lease_admin_address="$6"
  local -r admin_contract_address="$7"
  local -r wasm_path="$8"
  local -r network="$9"
  local -r dex_name="${10}"
  local -r dex_type_and_params="${11}"
  local -r dex_connection="${12}"
  local -r dex_channel_local="${13}"
  local -r dex_channel_remote="${14}"
  local -r protocol_currency="${15}"
  local -r treasury_contract_address="${16}"
  local -r timealarms_contract_address="${17}"
  local swap_tree="${18}"
  swap_tree="$(echo "$swap_tree" | sed 's/^"\(.*\)"$/\1/')"

  to_screaming_snake_case() {
    echo "${1}" \
      | sed "s/\([[:lower:]]\)\([[:upper:]]\)/\1_\2/g" \
      | tr "[:lower:]" "[:upper:]"
  }

  local -r protocol="$(to_screaming_snake_case "${network}-${dex_name}-${protocol_currency}")"

  unset -f to_screaming_snake_case

  FLAGS="--broadcast-mode sync --fees 200000unls --gas auto --gas-adjustment 1.2 --node $nolus_net --chain-id $chain_id"

  local are_valid
  if ! are_valid=$(_keys_check "$nolus_net" "$nolus_home_dir" "$dex_admin_wallet_key" "$store_code_privileged_wallet_key");  then
    echo "Error: Failed to get address for key $dex_admin_wallet_key"
    exit 1
  fi

  if [ "$are_valid" = "true" ]; then
    # upload Leaser code
    local -r leaser_code_id=$(store_code "$nolus_net" "$nolus_home_dir" "$store_code_privileged_wallet_key" "$wasm_path/leaser.wasm" "--instantiate-anyof-addresses $admin_contract_address")

    # upload Lease code
    local -r leaser_expected_address=$(_get_predictable_contract_address "$nolus_net" "$nolus_home_dir" "$admin_contract_address" "$leaser_code_id" "$protocol")
    local -r lease_code_id=$(store_code "$nolus_net" "$nolus_home_dir" "$store_code_privileged_wallet_key" "$wasm_path/lease.wasm" "--instantiate-anyof-addresses $leaser_expected_address")

    # upload and instantiate LPP
    local -r lpp_init_msg='{"lease_code_admin":"'"$leaser_expected_address"'","lease_code":"'"$lease_code_id"'","borrow_rate":{"base_interest_rate":60,"utilization_optimal":700,"addon_optimal_interest_rate":20},"min_utilization":0}'
    local -r lpp_contract_address=$(_deploy_contract "$nolus_net" "$nolus_home_dir" "$dex_admin_wallet_key" "$store_code_privileged_wallet_key" "$admin_contract_address" "$wasm_path/lpp.wasm" "$lpp_init_msg" "$protocol-lpp" "$protocol")

    # upload and instantiate Oracle
    local -r oracle_init_msg='{"config":{"price_config":{"min_feeders":500,"sample_period_secs":50,"samples_number":12,"discount_factor":650}},"swap_tree":'"$swap_tree"'}'
    local -r oracle_contract_address=$(_deploy_contract "$nolus_net" "$nolus_home_dir" "$dex_admin_wallet_key" "$store_code_privileged_wallet_key" "$admin_contract_address" "$wasm_path/oracle.wasm" "$oracle_init_msg" "$protocol-oracle" "$protocol")

    # upload and instantiate Profit
    local -r profit_init_msg='{"cadence_hours":12,"treasury":"'"$treasury_contract_address"'","oracle":"'"$oracle_contract_address"'","timealarms":"'"$timealarms_contract_address"'","dex":{"connection_id":"'"$dex_connection"'","transfer_channel":{"local_endpoint":"'"$dex_channel_local"'","remote_endpoint":"'"$dex_channel_remote"'"}}}'
    local -r profit_contract_address=$(_deploy_contract "$nolus_net" "$nolus_home_dir" "$dex_admin_wallet_key" "$store_code_privileged_wallet_key" "$admin_contract_address" "$wasm_path/profit.wasm" "$profit_init_msg" "$protocol-profit" "$protocol")

    # upload and instantiate Reserve
    local -r reserve_init_msg='{"lease_code_admin":"'"$leaser_expected_address"'","lease_code":"'"$lease_code_id"'"}'
    local -r reserve_contract_address=$(_deploy_contract "$nolus_net" "$nolus_home_dir" "$dex_admin_wallet_key" "$store_code_privileged_wallet_key" "$admin_contract_address" "$wasm_path/reserve.wasm" "$reserve_init_msg" "$protocol-reserve" "$protocol")

    # instantiate Leaser
    local -r leaser_init_msg='{"lease_code":"'"$lease_code_id"'","lpp":"'"$lpp_contract_address"'","profit":"'"$profit_contract_address"'","reserve":"'"$reserve_contract_address"'","time_alarms":"'"$timealarms_contract_address"'","market_price_oracle":"'"$oracle_contract_address"'","protocols_registry":"'"$admin_contract_address"'","lease_position_spec":{"liability":{"initial":600,"healthy":830,"first_liq_warn":850,"second_liq_warn":865,"third_liq_warn":880,"max":900,"recalc_time":432000000000000},"min_asset":{"amount":"150","ticker":"'"$protocol_currency"'"},"min_transaction":{"amount":"10","ticker":"'"$protocol_currency"'"}},"lease_interest_rate_margin":20,"lease_due_period":2592000000000000,"dex":{"connection_id":"'"$dex_connection"'","transfer_channel":{"local_endpoint":"'"$dex_channel_local"'","remote_endpoint":"'"$dex_channel_remote"'"}},"lease_max_slippages":{"liquidation":300},"lease_admin":"'"$lease_admin_address"'"}'
    local -r leaser_contract_address=$(_instantiate "$nolus_net" "$nolus_home_dir" "$dex_admin_wallet_key" "$leaser_code_id" "$leaser_init_msg" "$protocol-leaser" "$protocol" "$leaser_expected_address" "$admin_contract_address")

    # register the protocol
    local -r add_protocol_set_exec_msg='{"register_protocol":{"name":"'"$protocol"'","protocol":{"network":"'"$network"'","dex":'"$dex_type_and_params"',"contracts":{"leaser":"'"$leaser_contract_address"'","lpp":"'"$lpp_contract_address"'","oracle":"'"$oracle_contract_address"'","profit":"'"$profit_contract_address"'","reserve":"'"$reserve_contract_address"'"}}}}'
    run_cmd "$nolus_home_dir" tx wasm execute "$admin_contract_address" "$add_protocol_set_exec_msg" --from "$dex_admin_wallet_key" $FLAGS --yes
  fi
}

if [ "$#" -ne 0 ]; then
  case "$1" in
    deploy_contracts)
      shift
      deploy_contracts "$1" "$2" "$3" "$4" "$5" "$6" "$7" "$8" "$9" "${10}" "${11}" "${12}" "${13}" "${14}" "${15}" "${16}" "${17}" "${18}"
      ;;
    *)
      echo "Unknown function: $1"
      exit 1
      ;;
  esac
fi
