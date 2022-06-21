#!/bin/bash

# The functions require 'run_cmd' funcion to be available in the shell

add_wasm_messages() {
local -r genesis_home_dir="$1"
local -r wasm_code_path="$2"
local -r admin_addr="$3"
local -r treasury_init_tokens="$4"
local -r stable_denom="$5"

local -r LEASE_CODE_ID=2
local -r TREASURY_ADDRESS=$(treasury_instance_addr)
local -r LPP_ADDRESS="nolus1qg5ega6dykkxc307y25pecuufrjkxkaggkkxh7nad0vhyhtuhw3sqaa3c5"
local -r ORACLE_ADDRESS="nolus1436kxs0w2es6xlqpp9rd35e3d0cjnw4sv8j3a7483sgks29jqwgsv3wzl4"

local id=0

local -r treasury_init_msg='{}'
_add_wasm_message "$genesis_home_dir" "$wasm_code_path" "treasury" "$((++id))" "$admin_addr" "$treasury_init_tokens" "$treasury_init_msg"

_add_wasm_message "$genesis_home_dir" "$wasm_code_path" "lease" "$((++id))" "$admin_addr" ""

local -r lpp_init_msg='{"denom":"'$stable_denom'","lease_code_id":"'$LEASE_CODE_ID'"}'
_add_wasm_message "$genesis_home_dir" "$wasm_code_path" "lpp" "$((++id))" "$admin_addr" "" "$lpp_init_msg"

local -r leaser_init_msg='{"lease_code_id":"'$LEASE_CODE_ID'","lease_interest_rate_margin":30,"recalc_hours":2,"liability":{"healthy":70,"initial":65,"max":80},"lpp_ust_addr":"'$LPP_ADDRESS'","repayment":{"grace_period_sec":864000,"period_sec":5184000}}'
_add_wasm_message "$genesis_home_dir" "$wasm_code_path" "leaser" "$((++id))"  "$admin_addr" "" "$leaser_init_msg"

local -r oracle_init_msg='{"base_asset":"'$stable_denom'","price_feed_period":60,"feeders_percentage_needed":50,"supported_denom_pairs":[["OSMO","'$stable_denom'"],["LUNA","OSMO"],["IRIS","OSMO"]]}'
_add_wasm_message "$genesis_home_dir" "$wasm_code_path" "oracle" "$((++id))" "$admin_addr" "" "$oracle_init_msg"

local -r profit_init_msg='{"cadence_hours":7200,"treasury":"'$TREASURY_ADDRESS'","time_oracle":"'$ORACLE_ADDRESS'"}'
_add_wasm_message "$genesis_home_dir" "$wasm_code_path" "profit" "$((++id))" "$admin_addr" "" "$profit_init_msg"

local -r dispatcher_init_msg='{"cadence_hours":7200,"lpp":"'$LPP_ADDRESS'","time_oracle":"'$ORACLE_ADDRESS'","treasury":"'$TREASURY_ADDRESS'","market_oracle":"'$ORACLE_ADDRESS'","tvl_to_apr":{"intervals":[{"tvl":0,"apr":300},{"tvl":1000,"apr":90},{"tvl":1000000,"apr":30}]}}';
_add_wasm_message "$genesis_home_dir" "$wasm_code_path" "rewards_dispatcher" "$((++id))" "$admin_addr" "" "$dispatcher_init_msg"

}

treasury_instance_addr() {
  # An instance address is computed as a function of the code ID and the globally incremented number of instantiations done so far.
  # A consequence of the above is that the instance address of smart contracts will not change when the code binary changes
  # unless the order is changed.

  # this the address of the first instatiation that is of the first deployed code, assuming that is treasury.
  # to update if the order is changed
  echo "nolus14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9s0k0puz"
}

_add_wasm_message() {
local -r genesis_home_dir="$1"
local -r wasm_code_path="$2"
local -r contract_name="$3"
local -r code_id="$4"
local -r admin_addr="$5"
local -r init_tokens="$6"

if ! [ -f "$wasm_code_path/$contract_name.wasm" ]; then
  echo "The path '$wasm_code_path' does not contain the $contract_name contracts' code."
  exit 1
fi

local amount_flag=""
if ! [ "$init_tokens" = "" ]; then
    amount_flag="--amount $init_tokens"
fi

run_cmd "$genesis_home_dir" add-wasm-genesis-message store "$wasm_code_path/$contract_name.wasm" --run-as "$admin_addr"

if [[ $# -eq 7 ]]; then
  local -r init_msg="$7"

  run_cmd "$genesis_home_dir" add-wasm-genesis-message instantiate-contract "$code_id" "$init_msg" --label "$contract_name" \
    --run-as "$admin_addr" --admin "$admin_addr" $amount_flag
fi
}