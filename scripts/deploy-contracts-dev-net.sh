#!/bin/bash
set -euxo pipefail

ROOT_DIR=$(pwd)
ARTIFACT_BIN="nolus.tar.gz"
NOLUS_DEV_NET="https://net-dev.nolus.io:26612"
ACCOUNTS_DIR="$(pwd)/accounts"
export TXFLAG="--gas-prices 0.025unolus --gas auto --gas-adjustment 1.3 -y --home $ACCOUNTS_DIR --node $NOLUS_DEV_NET"

deployContract() {

    RES=$(nolusd tx wasm store artifacts/$1.wasm --from treasury ${TXFLAG} --output json -b block)
    CODE_ID=$(echo $RES | jq -r '.logs[0].events[-1].attributes[0].value')

    #nolusd tx wasm instantiate $CODE_ID "$2" --from treasury --label "$1" ${TXFLAG} --no-admin
    #  sleep 6
    #CONTRACT_ADDRESS=$(nolusd query wasm list-contract-by-code $CODE_ID --node $NOLUS_DEV_NET --output json | jq -r '.contracts[-1]')

    if [[ ! -e "contracts-addresses" ]]; then
        mkdir "contracts-addresses"
    fi

    if [[ -d "contracts-addresses/$1" ]]; then
        rm -rf contracts-addresses/$1
    fi

    cd contracts/$1
    cargo schema

    mkdir $ROOT_DIR/contracts-addresses/$1
    cp -R schema $ROOT_DIR/contracts-addresses/$1


#CONTRACT_ADDRESS=${CONTRACT_ADDRESS}
INFO=$(cat <<-EOF
CODE_ID=${CODE_ID}
EOF
)
echo "$INFO" > "$ROOT_DIR/contracts-addresses/$1/info"

cd $ROOT_DIR
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

# if we need CONTRACT_ADDRESS var we will send init msg to deployContract():
#INIT_MSG_ORACLE='{"base_asset":"ust","price_feed_period":60,"feeders_percentage_needed":50}'
deployContract "oracle"
deployContract "loan"
deployContract "treasury"
deployContract "borrow"
