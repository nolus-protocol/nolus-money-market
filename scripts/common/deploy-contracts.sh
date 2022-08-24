#!/bin/bash
set -euxo pipefail
SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
source "$SCRIPT_DIR"/cmd.sh

deployContract() {
  local -r contracts_results_file="$1"
  local -r nolus_net="$2"
  local -r home_dir="$3"
  local -r contract_name="$4"
  local -r txflags="--gas-prices 0.025unolus --gas auto --gas-adjustment 1.3 -y --node $nolus_net"

 local -r res=$(run_cmd "$home_dir" tx wasm store artifacts/"$contract_name".wasm --from treasury $txflags --output json -b "block")
 local -r fix_response='{"height"'${res#*'{"height"'}
 local -r code_id=$(echo "$fix_response" | jq -r '.logs[0].events[-1].attributes[0].value')

  if [[ $# -eq 4 ]]; then
    local -r info='{"'$contract_name'":{"code_id":"'$code_id'"}}'
  else
    local -r init_msg="$5"
    run_cmd "$home_dir" tx wasm instantiate "$code_id" "$init_msg" --from treasury --label "$contract_name" $txflags --no-admin -b "block"
    local -r contract_address=$(run_cmd "$home_dir" query wasm list-contract-by-code "$code_id" --node "$nolus_net" --output json | jq -r '.contracts[-1]')
    local -r info='{"'$contract_name'":{"instance":"'$contract_address'","code_id":"'$code_id'"}}'
  fi
  jq --argjson contract_info "$info" '.contracts_info |= . + [$contract_info]' "$contracts_results_file" > tmp.json && mv tmp.json "$contracts_results_file"
}

deployContracts() {
local -r contracts_results_file="$1"
local -r nolus_net="$2"
local -r home_dir="$3"
local -r stable_denom="$4"

jq -n '{"contracts_info":[]}' > "$contracts_results_file"

deployContract "$contracts_results_file" "$nolus_net" "$home_dir" "lease"
local -r lease_code_id=$(jq .contracts_info[0].lease.code_id contracts-info.json | tr -d '"')

local -r lpp_init_msg='{"denom":"'$stable_denom'","lease_code_id":"'$lease_code_id'"}'
deployContract "$contracts_results_file" "$nolus_net" "$home_dir" "lpp"  "$lpp_init_msg"
local -r lpp_address=$(jq .contracts_info[1].lpp.instance contracts-info.json | tr -d '"')

local -r leaser_init_msg='{"lease_code_id":"'$lease_code_id'","lease_interest_rate_margin":30,"recalc_hours":2,"liability":{"healthy":70,"initial":65,"max":80,"first_liq_warn":72,"second_liq_warn":75,"third_liq_warn":78},"lpp_ust_addr":"'$lpp_address'","repayment":{"grace_period_sec":864000,"period_sec":5184000}}'
deployContract "$contracts_results_file" "$nolus_net" "$home_dir" "leaser" "$leaser_init_msg"

local -r oracle_init_msg='{"base_asset":"'$stable_denom'","price_feed_period":60,"feeders_percentage_needed":50,"supported_denom_pairs":[["OSMO","'$stable_denom'"],["LUNA","OSMO"],["IRIS","OSMO"]]}'
deployContract "$contracts_results_file" "$nolus_net" "$home_dir" "oracle" "$oracle_init_msg"
local -r oracle_address=$(jq .contracts_info[3].oracle.instance contracts-info.json | tr -d '"')

local -r treasury_init_msg='{}'
deployContract "$contracts_results_file" "$nolus_net" "$home_dir" "treasury" "$treasury_init_msg"
local -r treasury_address=$(jq .contracts_info[4].treasury.instance contracts-info.json | tr -d '"')

local -r profit_init_msg='{"cadence_hours":7200,"treasury":"'$treasury_address'","time_oracle":"'$oracle_address'"}'
deployContract "$contracts_results_file" "$nolus_net" "$home_dir" "profit" "$profit_init_msg"

local -r dispatcher_init_msg='{"cadence_hours":7200,"lpp":"'$lpp_address'","time_oracle":"'$oracle_address'","treasury":"'$treasury_address'","market_oracle":"'$oracle_address'","tvl_to_apr":{"intervals":[{"tvl":0,"apr":300},{"tvl":1000,"apr":90},{"tvl":1000000,"apr":30}]}}';
deployContract "$contracts_results_file" "$nolus_net" "$home_dir" "rewards_dispatcher" "$dispatcher_init_msg"
}
