#!/bin/bash
set -euxo pipefail

ARTIFACT_BIN="nolus.tar.gz"
NOLUS_DEV_NET="https://net-dev.nolus.io:26612"
ACCOUNTS_DIR="$(pwd)/accounts"
export TXFLAG="--gas-prices 0.025unolus --gas auto --gas-adjustment 1.3 -y --home $ACCOUNTS_DIR --node $NOLUS_DEV_NET"

deployContract() {

    RES=$(nolusd tx wasm store artifacts/$1.wasm --from treasury ${TXFLAG} --output json -b block)
    CODE_ID=$(echo $RES | jq -r '.logs[0].events[-1].attributes[0].value')

    nolusd tx wasm instantiate $CODE_ID "$2" --from treasury --label "$1" ${TXFLAG} --no-admin
    sleep 6
    CONTRACT_ADDRESS=$(nolusd query wasm list-contract-by-code $CODE_ID --node $NOLUS_DEV_NET --output json | jq -r '.contracts[-1]')

    if [[ ! -e "contracts-addresses" ]]; then
        mkdir "contracts-addresses"
    fi

PAIR=$(cat <<-EOF
CODE_ID=${CODE_ID}
CONTRACT_ADDRESS=${CONTRACT_ADDRESS}
EOF
)
echo "$PAIR" > "contracts-addresses/$1"
}


if [[ $# -eq 0 ]]; then
 if [[ -z ${CI_JOB_TOKEN+x} ]]; then
    echo "Error: there is no PRIVATE or CI_JOB token"
    exit 1
  else
    TOKEN_TYPE="JOB-TOKEN"
    TOKEN_VALUE="$CI_JOB_TOKEN"
  fi
else
  TOKEN_TYPE="PRIVATE-TOKEN"
  TOKEN_VALUE="$1"
fi

VERSION=$(curl --silent "$NOLUS_DEV_NET/abci_info" | jq '.result.response.version' | tr -d '"')

curl --output binary.zip --header "$TOKEN_TYPE: $TOKEN_VALUE" "https://gitlab-nomo.credissimo.net/api/v4/projects/3/jobs/artifacts/v$VERSION/download?job=build-binary"
echo 'A' | unzip binary.zip
tar -xf $ARTIFACT_BIN
export PATH=$(pwd):$PATH

curl --output artifacts.zip --header "$TOKEN_TYPE: $TOKEN_VALUE" "https://gitlab-nomo.credissimo.net/api/v4/projects/3/jobs/artifacts/v$VERSION/download?job=setup-dev-network"
echo 'A' | unzip artifacts.zip

# deploy all contracts
INIT_MSG_ORACLE='{"base_asset":"ust","price_feed_period":60,"feeders_percentage_needed":50}'
deployContract "oracle" $INIT_MSG_ORACLE

