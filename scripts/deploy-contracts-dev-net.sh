#!/bin/bash
set -euxo pipefail

if [[ -z ${CI_JOB_TOKEN+x} ]]; then
  echo "Error: there is no CI_JOB token"
  exit 1
fi

TOKEN_TYPE="JOB-TOKEN"
TOKEN_VALUE="$CI_JOB_TOKEN"

BINARY_ARTIFACT_BIN="nolus.tar.gz"
NOLUS_DEV_NET="https://net-dev.nolus.io:26612"
GITLAB_API="https://gitlab-nomo.credissimo.net/api/v4"
COSMZONE_PROJECT_ID="3"
SETUP_DEV_NETWORK_ARTIFACT="setup-dev-network"
NOLUS_BUILD_BINARY_ARTIFACT="build-binary"
ACCOUNTS_DIR="$(pwd)/accounts"
TXFLAG="--gas-prices 0.025unolus --gas auto --gas-adjustment 1.3 -y --home $ACCOUNTS_DIR --node $NOLUS_DEV_NET"
CONTRACTS_RESULTS_FILE="$1"

# init msgs
ORACLE_INIT_MSG='{"base_asset":"ust","price_feed_period":60,"feeders_percentage_needed":50}'
LEASER_INIT_MSG='{"grace_period_nano_sec":"23","loan_code_id":1,"loan_healthy_liability":"1.584007913129639935","loan_interest_rate_margin":"30.584007913129639935","loan_max_liability":"4.584007913129639935","lpp_ust_addr":"nolus1qaf23chpkknx2znmz6p7k7n0u2uk5xtr5zdaf2","repayment_period_nano_sec":"50"}'
TREASURY_INIT_MSG='{}'
LPP_INIT_MSG='{"denom":"ust","lease_code_id":1}'

downloadArtifact() {
  local name="$1"
  local version="$2"
  local project_id="$3"

  curl --output "$name".zip --header "$TOKEN_TYPE: $TOKEN_VALUE" "$GITLAB_API/projects/$project_id/jobs/artifacts/v$version/download?job=$name"
  echo 'A' | unzip "$name".zip
}

deployContract() {
  local contract_name="$1"

  RES=$(nolusd tx wasm store artifacts/"$contract_name".wasm --from treasury $TXFLAG --output json -b block)
  CODE_ID=$(echo "$RES" | jq -r '.logs[0].events[-1].attributes[0].value')

  if [[ $# -eq 1 ]]; then
    local info='{"'$contract_name'":{"code_id":"'$CODE_ID'"}}'
  else
    local init_msg="$2"
    nolusd tx wasm instantiate "$CODE_ID" "$init_msg" --from treasury --label "$contract_name" $TXFLAG --no-admin -b "block"
    CONTRACT_ADDRESS=$(nolusd query wasm list-contract-by-code "$CODE_ID" --node "$NOLUS_DEV_NET" --output json | jq -r '.contracts[-1]')
    local info='{"'$contract_name'":{"instance":"'$CONTRACT_ADDRESS'","code_id":"'$CODE_ID'"}}'
  fi
  jq --argjson contract_info "$info" '.contracts_info |= . + [$contract_info]' "$CONTRACTS_RESULTS_FILE" > tmp.json && mv tmp.json "$CONTRACTS_RESULTS_FILE"
}

# Download the build-binary and setup-dev-network artifacts from cosmozone
VERSION=$(curl --silent "$NOLUS_DEV_NET/abci_info" | jq '.result.response.version' | tr -d '"')
downloadArtifact "$SETUP_DEV_NETWORK_ARTIFACT" "$VERSION" "$COSMZONE_PROJECT_ID"
downloadArtifact "$NOLUS_BUILD_BINARY_ARTIFACT" "$VERSION" "$COSMZONE_PROJECT_ID"
tar -xf $BINARY_ARTIFACT_BIN

export PATH;
PATH=$(pwd):$PATH

jq -n '{"contracts_info":[]}' > "$CONTRACTS_RESULTS_FILE"

# Deploy smart contracts
deployContract "oracle" "$ORACLE_INIT_MSG"
deployContract "leaser" "$LEASER_INIT_MSG"
deployContract "lease"
deployContract "treasury" "$TREASURY_INIT_MSG"
deployContract "lpp" "$LPP_INIT_MSG"
