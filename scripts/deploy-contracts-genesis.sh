#!/bin/bash

# The functions require 'run_cmd' function to be available in the shell

add_wasm_messages() {
  local -r genesis_home_dir="$1"
  local -r wasm_code_path="$2"
  local -r treasury_init_tokens="$3"
  local -r protocol_currency="$4"
  local -r contracts_info_file="$5"
  local -r dex_admin="$6"
  local -r protocol_identifier="$7"
  local -r swap_tree="$8"

  local -r LEASE_CODE_ID=2
  local -r TREASURY_ADDRESS=$(treasury_instance_addr)
  local -r LPP_ADDRESS=$(lpp_instance_addr)
  local -r TIMEALARMS_ADDRESS=$(timealarms_instance_addr)
  local -r ORACLE_ADDRESS=$(oracle_instance_addr)
  local -r LEASER_ADDRESS=$(leaser_instance_addr)
  local -r PROFIT_ADDRESS=$(profit_instance_addr)
  local -r REWARDS_DISPATCHER_ADDRESS=$(rewards_dispatcher_instance_addr)
  local -r ADMIN_CONTRACT_ADDRESS=$(admin_contract_instance_addr)

  jq -n '{"contracts_info":[]}' > "$contracts_info_file"

  local id=0

  local -r treasury_init_msg='{"rewards_dispatcher":"'"$REWARDS_DISPATCHER_ADDRESS"'"}'
  _add_wasm_message "$genesis_home_dir" "$wasm_code_path" "treasury" "$((++id))" \
    "$ADMIN_CONTRACT_ADDRESS" "$treasury_init_tokens"  "--instantiate-anyof-addresses $ADMIN_CONTRACT_ADDRESS" \
    "$treasury_init_msg"
  _export_to_file "treasury" "$TREASURY_ADDRESS" "$contracts_info_file"

  _add_wasm_message "$genesis_home_dir" "$wasm_code_path" "lease" "$((++id))" \
    "$ADMIN_CONTRACT_ADDRESS" "" "--instantiate-anyof-addresses $LEASER_ADDRESS"

  local -r lpp_init_msg='{"lpn_ticker":"'"$protocol_currency"'","lease_code_admin":"'"$LEASER_ADDRESS"'","borrow_rate":{"base_interest_rate":100,"utilization_optimal":750,"addon_optimal_interest_rate":20},"min_utilization":0}'
  _add_wasm_message "$genesis_home_dir" "$wasm_code_path" "lpp" "$((++id))" \
    "$ADMIN_CONTRACT_ADDRESS" "" "--instantiate-anyof-addresses $ADMIN_CONTRACT_ADDRESS" "$lpp_init_msg"
  _export_to_file "lpp" "$LPP_ADDRESS" "$contracts_info_file"

  local -r timealarms_init_msg='{}'
  _add_wasm_message "$genesis_home_dir" "$wasm_code_path" "timealarms" "$((++id))" \
    "$ADMIN_CONTRACT_ADDRESS" "" "--instantiate-anyof-addresses $ADMIN_CONTRACT_ADDRESS" "$timealarms_init_msg"
  _export_to_file "timealarms" "$TIMEALARMS_ADDRESS" "$contracts_info_file"

  local -r oracle_init_msg='{"config":{"base_asset":"'"$protocol_currency"'","price_config":{"min_feeders":500,"sample_period_secs":10,"samples_number":12,"discount_factor":750}},"swap_tree":'"$swap_tree"'}'
  _add_wasm_message "$genesis_home_dir" "$wasm_code_path" "oracle" "$((++id))" \
    "$ADMIN_CONTRACT_ADDRESS" "" "--instantiate-anyof-addresses $ADMIN_CONTRACT_ADDRESS" "$oracle_init_msg"
  _export_to_file "oracle" "$ORACLE_ADDRESS" "$contracts_info_file"

  local -r profit_init_msg='{"cadence_hours":7200,"treasury":"'"$TREASURY_ADDRESS"'","oracle":"'"$ORACLE_ADDRESS"'","timealarms":"'"$TIMEALARMS_ADDRESS"'"}'
  _add_wasm_message "$genesis_home_dir" "$wasm_code_path" "profit" "$((++id))" \
    "$ADMIN_CONTRACT_ADDRESS" "" "--instantiate-anyof-addresses $ADMIN_CONTRACT_ADDRESS" "$profit_init_msg"
  _export_to_file "profit" "$PROFIT_ADDRESS" "$contracts_info_file"

  local -r leaser_init_msg='{"lease_code_id":"'"$LEASE_CODE_ID"'","lease_interest_rate_margin":30,"lease_position_spec":{"liability":{"initial":650,"healthy":700,"first_liq_warn":720,"second_liq_warn":750,"third_liq_warn":780,"max":800,"recalc_time":7200000000000},"min_asset":{"amount":"15000","ticker":"'"$protocol_currency"'"},"min_transaction":{"amount":"1000","ticker":"'"$protocol_currency"'"}},"lpp_ust_addr":"'"$LPP_ADDRESS"'","time_alarms":"'"$TIMEALARMS_ADDRESS"'","market_price_oracle":"'"$ORACLE_ADDRESS"'","profit":"'"$PROFIT_ADDRESS"'","lease_interest_payment":{"due_period":5184000000000000,"grace_period":864000000000000}}'
  _add_wasm_message "$genesis_home_dir" "$wasm_code_path" "leaser" "$((++id))" \
    "$ADMIN_CONTRACT_ADDRESS" "" "--instantiate-anyof-addresses $ADMIN_CONTRACT_ADDRESS" "$leaser_init_msg"
  _export_to_file "leaser" "$LEASER_ADDRESS" "$contracts_info_file"

  local -r dispatcher_init_msg='{"cadence_hours":12,"protocols_registry":"'"$ADMIN_CONTRACT_ADDRESS"'","timealarms":"'"$TIMEALARMS_ADDRESS"'","treasury":"'"$TREASURY_ADDRESS"'","tvl_to_apr":{"bars":[{"tvl":0,"apr":150},{"tvl":500,"apr":140},{"tvl":1000,"apr":130},{"tvl":2000,"apr":120},{"tvl":3000,"apr":110},{"tvl":4000,"apr":100},{"tvl":5000,"apr":90},{"tvl":7500,"apr":80},{"tvl":10000,"apr":70},{"tvl":15000,"apr":60},{"tvl":20000,"apr":50},{"tvl":25000,"apr":40},{"tvl":30000,"apr":30},{"tvl":40000,"apr":20}]}}'
  _add_wasm_message "$genesis_home_dir" "$wasm_code_path" "rewards_dispatcher" \
    "$((++id))" "$ADMIN_CONTRACT_ADDRESS" "" "--instantiate-anyof-addresses $ADMIN_CONTRACT_ADDRESS" \
    "$dispatcher_init_msg"
  _export_to_file "rewards_dispatcher" "$REWARDS_DISPATCHER_ADDRESS" \
    "$contracts_info_file"

  local -r admin_contract_init_msg='{"dex_admin":"'"$dex_admin"'","contracts":{"platform":{"dispatcher":"'"$REWARDS_DISPATCHER_ADDRESS"'","timealarms":"'"$TIMEALARMS_ADDRESS"'","treasury":"'"$TREASURY_ADDRESS"'"},"protocol":{"'"$protocol_identifier"'":{"leaser":"'"$LEASER_ADDRESS"'","lpp":"'"$LPP_ADDRESS"'","oracle":"'"$ORACLE_ADDRESS"'","profit":"'"$PROFIT_ADDRESS"'"}}}}'
  _add_wasm_message "$genesis_home_dir" "$wasm_code_path" "admin_contract" \
    "$((++id))" "$ADMIN_CONTRACT_ADDRESS" "" "--instantiate-anyof-addresses $ADMIN_CONTRACT_ADDRESS" \
    "$admin_contract_init_msg"
  _export_to_file "admin_contract" "$ADMIN_CONTRACT_ADDRESS" "$contracts_info_file"
}

treasury_instance_addr() {
  # An instance address is computed as a function of the code ID and the globally
  # incremented number of instantiations done so far.
  # A consequence of the above is that the instance address of smart contracts
  # will not change when the code binary changes unless the order is changed.

  # this the address of the first instatiation that is of the first deployed
  # code, assuming that is treasury.
  # to update if the order is changed
  echo "nolus14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9s0k0puz"
}

lpp_instance_addr() {
  echo "nolus1qg5ega6dykkxc307y25pecuufrjkxkaggkkxh7nad0vhyhtuhw3sqaa3c5"
}

timealarms_instance_addr() {
  echo "nolus1zwv6feuzhy6a9wekh96cd57lsarmqlwxdypdsplw6zhfncqw6ftqmx7chl"
}

oracle_instance_addr() {
  echo "nolus1436kxs0w2es6xlqpp9rd35e3d0cjnw4sv8j3a7483sgks29jqwgsv3wzl4"
}

profit_instance_addr() {
  echo "nolus1mf6ptkssddfmxvhdx0ech0k03ktp6kf9yk59renau2gvht3nq2gqkxgywu"
}

leaser_instance_addr() {
  echo "nolus1wn625s4jcmvk0szpl85rj5azkfc6suyvf75q6vrddscjdphtve8s5gg42f"
}

rewards_dispatcher_instance_addr() {
  echo "nolus1tqwwyth34550lg2437m05mjnjp8w7h5ka7m70jtzpxn4uh2ktsmqtctwnn"
}

admin_contract_instance_addr() {
  echo "nolus1gurgpv8savnfw66lckwzn4zk7fp394lpe667dhu7aw48u40lj6jsqxf8nd"
}

_export_to_file() {
  local -r contract_name="$1"
  local -r contract_address="$2"
  local -r contracts_info_file="$3"

  local -r info='{"'$contract_name'":{"instance":"'$contract_address'"}}'
  jq --argjson contract_info "$info" '.contracts_info |= . + [$contract_info]' \
    "$contracts_info_file" > tmp.json && mv tmp.json "$contracts_info_file"
}

_add_wasm_message() {
  local -r genesis_home_dir="$1"
  local -r wasm_code_path="$2"
  local -r contract_name="$3"
  local -r code_id="$4"
  local -r admin_addr="$5"
  local -r init_tokens="$6"
  local -r instantiate_policy="$7"

  if ! [ -f "$wasm_code_path/$contract_name.wasm" ]; then
    echo "The path '$wasm_code_path' does not contain the $contract_name " \
      "contracts' code."
    exit 1
  fi

  local amount_flag=""
  if ! [ "$init_tokens" = "" ]; then
      amount_flag="--amount $init_tokens"
  fi

  run_cmd "$genesis_home_dir" "add-wasm-genesis-message" "store" \
    "$wasm_code_path/$contract_name.wasm" "--run-as" "$admin_addr" \
    $instantiate_policy

  if [[ $# -eq 8 ]]; then
    local -r init_msg="$8"

    run_cmd "$genesis_home_dir" "add-wasm-genesis-message" "instantiate-contract" \
      "$code_id" "$init_msg" "--label" "$contract_name" "--run-as" "$admin_addr" \
      "--admin" "$admin_addr" $amount_flag
  fi
}
