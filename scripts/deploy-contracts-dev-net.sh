#!/bin/bash
set -euxo pipefail

if [[ -z ${CI_JOB_TOKEN+x} ]]; then
  echo "Error: there is no CI_JOB token"
  exit 1
else
  TOKEN_TYPE="JOB-TOKEN"
  TOKEN_VALUE="$CI_JOB_TOKEN"
fi

ROOT_DIR=$(pwd)
BINARY_ARTIFACT_BIN="nolus.tar.gz"
NOLUS_DEV_NET="https://net-dev.nolus.io:26612"
GITLAB_API="https://gitlab-nomo.credissimo.net/api/v4"
ACCOUNTS_DIR="$(pwd)/accounts"
TXFLAG="--gas-prices 0.025unolus --gas auto --gas-adjustment 1.3 -y --home $ACCOUNTS_DIR --node $NOLUS_DEV_NET"

# init msgs
ORACLE_INIT_MSG='{"base_asset":"ust","price_feed_period":60,"feeders_percentage_needed":50}'
BORROW_INIT_MSG='{"grace_period_nano_sec":"23","loan_code_id":1,"loan_healthy_liability":"1.584007913129639935","loan_interest_rate_margin":"30.584007913129639935","loan_max_liability":"4.584007913129639935","lpp_ust_addr":"nolus1qaf23chpkknx2znmz6p7k7n0u2uk5xtr5zdaf2","repayment_period_nano_sec":"50"}'
LOAN_INIT_MSG='{"owner":"nolus1qaf23chpkknx2znmz6p7k7n0u2uk5xtr5zdaf2"}'
TREASURY_INIT_MSG='{}'

downloadArtifact() {
  local name="$1"
  local version="$2"

  curl --output "$name".zip --header "$TOKEN_TYPE: $TOKEN_VALUE" "$GITLAB_API/projects/3/jobs/artifacts/v$version/download?job=$name"
  echo 'A' | unzip "$name".zip
}

deployContract() {
  local contract_name="$1"
  local init_msg="$2"

  RES=$(nolusd tx wasm store artifacts/"$contract_name".wasm --from treasury $TXFLAG --output json -b block)
  NEW_CODE_ID=$(echo "$RES" | jq -r '.logs[0].events[-1].attributes[0].value')

  nolusd tx wasm instantiate "$NEW_CODE_ID" "$init_msg" --from treasury --label "$contract_name" $TXFLAG --no-admin -b "block"
  CONTRACT_ADDRESS=$(nolusd query wasm list-contract-by-code "$NEW_CODE_ID" --node "$NOLUS_DEV_NET" --output json | jq -r '.contracts[-1]')

  # prepare the results in contracts-results dir to be saved as artifact
  if [[ ! -e "contracts-results" ]]; then
      mkdir "contracts-results"
  fi

  if [[ -d "contracts-results/$contract_name" ]]; then
      rm -rf contracts-results/"$contract_name"
  fi

  mkdir "$ROOT_DIR"/contracts-results/"$contract_name"

INFO=$(cat <<-EOF
export CONTRACT_ADDRESS=${CONTRACT_ADDRESS}
export CODE_ID=${NEW_CODE_ID}
EOF
)
echo "$INFO" > "$ROOT_DIR/contracts-results/$contract_name/info.env"
  cd "$ROOT_DIR"
}

# Download the build-binary and setup-dev-network artifacts from cosmozone
VERSION=$(curl --silent "$NOLUS_DEV_NET/abci_info" | jq '.result.response.version' | tr -d '"')
downloadArtifact "setup-dev-network" "$VERSION"
downloadArtifact "build-binary" "$VERSION"
tar -xf $BINARY_ARTIFACT_BIN

export PATH;
PATH=$(pwd):$PATH

# Deploy smart contracts
deployContract "oracle" "$ORACLE_INIT_MSG"
deployContract "borrow" "$BORROW_INIT_MSG"
deployContract "loan" "$LOAN_INIT_MSG"
deployContract "treasury" "$TREASURY_INIT_MSG"
