#!/bin/bash
set -euxo pipefail

source "$COMMON_DIR/cmd.sh"

deployContract() {
  local contract_name="$1"
  local txflags="--gas-prices 0.025unolus --gas auto --gas-adjustment 1.3 -y --node $NOLUS_NET"

 RES=$(run_cmd "$HOME_DIR" tx wasm store artifacts/"$contract_name".wasm --from treasury $txflags --output json -b "block")
 FIX_RESPONSE='{"height"'${RES#*'{"height"'}
 CODE_ID=$(echo "$FIX_RESPONSE" | jq -r '.logs[0].events[-1].attributes[0].value')

  if [[ $# -eq 1 ]]; then
    local info='{"'$contract_name'":{"code_id":"'$CODE_ID'"}}'
  else
    local init_msg="$2"
    run_cmd "$HOME_DIR" tx wasm instantiate "$CODE_ID" "$init_msg" --from treasury --label "$contract_name" $txflags --no-admin -b "block"
    CONTRACT_ADDRESS=$(run_cmd "$HOME_DIR" query wasm list-contract-by-code "$CODE_ID" --node "$NOLUS_NET" --output json | jq -r '.contracts[-1]')
    local info='{"'$contract_name'":{"instance":"'$CONTRACT_ADDRESS'","code_id":"'$CODE_ID'"}}'
  fi
  jq --argjson contract_info "$info" '.contracts_info |= . + [$contract_info]' "$CONTRACTS_RESULTS_FILE" > tmp.json && mv tmp.json "$CONTRACTS_RESULTS_FILE"
}

deployContracts() {
CONTRACTS_RESULTS_FILE="$1"
NOLUS_NET="$2"
HOME_DIR="$3"
STABLE_DENOM="$4"

jq -n '{"contracts_info":[]}' > "$CONTRACTS_RESULTS_FILE"

deployContract "lease"
LEASE_CODE_ID=$(jq .contracts_info[0].lease.code_id contracts-info.json | tr -d '"')

LPP_INIT_MSG='{"denom":"'$STABLE_DENOM'","lease_code_id":"'$LEASE_CODE_ID'"}'
deployContract "lpp" "$LPP_INIT_MSG"
LPP_ADDRESS=$(jq .contracts_info[1].lpp.instance contracts-info.json | tr -d '"')

LEASER_INIT_MSG='{"lease_code_id":"'$LEASE_CODE_ID'","lease_interest_rate_margin":30,"recalc_hours":2,"liability":{"healthy":70,"initial":65,"max":80},"lpp_ust_addr":"'$LPP_ADDRESS'","repayment":{"grace_period_sec":864000,"period_sec":5184000}}'
deployContract "leaser" "$LEASER_INIT_MSG"

ORACLE_INIT_MSG='{"base_asset":"'$STABLE_DENOM'","price_feed_period":60,"feeders_percentage_needed":50,"supported_denom_pairs":[["OSMO","'$STABLE_DENOM'"],["LUNA","OSMO"],["IRIS","OSMO"]]}'
deployContract "oracle" "$ORACLE_INIT_MSG"
ORACLE_ADDRESS=$(jq .contracts_info[3].oracle.instance contracts-info.json | tr -d '"')

TREASURY_INIT_MSG='{}'
deployContract "treasury" "$TREASURY_INIT_MSG"
TREASURY_ADDRESS=$(jq .contracts_info[4].treasury.instance contracts-info.json | tr -d '"')


PROFIT_INIT_MSG='{"cadence_hours":7200,"treasury":"'$TREASURY_ADDRESS'","time_oracle":"'$ORACLE_ADDRESS'"}'
deployContract "profit" "$PROFIT_INIT_MSG"

DISPATCHER_INIT_MSG='{"cadence_hours":7200,"lpp":"'$LPP_ADDRESS'","time_oracle":"'$ORACLE_ADDRESS'","treasury":"'$TREASURY_ADDRESS'","market_oracle":"'$ORACLE_ADDRESS'","tvl_to_apr":{"intervals":[{"tvl":0,"apr":300},{"tvl":1000,"apr":90},{"tvl":1000000,"apr":30}]}}';
deployContract "rewards_dispatcher" "$DISPATCHER_INIT_MSG"
}