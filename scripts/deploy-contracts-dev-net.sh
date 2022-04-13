#!/bin/bash
set -euxo pipefail

ROOT_DIR=$(pwd)
BINARY_ARTIFACT_BIN="nolus.tar.gz"
CONTRACTS_ARTIFACT_BIN="contracts.tar.gz"

NOLUS_DEV_NET="https://net-dev.nolus.io:26612"
ACCOUNTS_DIR="$(pwd)/accounts"
export TXFLAG="--gas-prices 0.025unolus --gas auto --gas-adjustment 1.3 -y --home $ACCOUNTS_DIR --node $NOLUS_DEV_NET"
source msgs.env #load init msgs of contracts as env variables

deployContract() {

    RES=$(nolusd tx wasm store artifacts/$1.wasm --from treasury ${TXFLAG} --output json -b block)
    NEW_CODE_ID=$(echo $RES | jq -r '.logs[0].events[-1].attributes[0].value')

    # if there is no $1 dir in the latest version of deploy-contracts artifact -> this is a new contract, so we instantiate it
    if [[ ! -e "xxxxxlast-contracts-version/contracts-results/$1" ]]; then
        nolusd tx wasm instantiate $NEW_CODE_ID "$2" --from treasury --label "$1" ${TXFLAG} --no-admin
        sleep 6
        CONTRACT_ADDRESS=$(nolusd query wasm list-contract-by-code $NEW_CODE_ID --node $NOLUS_DEV_NET --output json | jq -r '.contracts[-1]')
    else # else this is an existing contract, so we migrate it
        source last-contracts-version/contracts-addresses/$1/info.env
        echo "migr"
        # nolusd tx wasm migrate ${CONTRACT_ADDRESS} $NEW_CODE_ID [json_encoded_migration_args] --from treasury
    fi

    # prepare the results in contracts-results dir to be saved as artifact
    if [[ ! -e "contracts-results" ]]; then
        mkdir "contracts-results"
    fi

    if [[ -d "contracts-results/$1" ]]; then
        rm -rf contracts-results/$1
    fi

    # generate schema
    cd contracts/$1
    cargo schema

    mkdir $ROOT_DIR/contracts-results/$1
    cp -R schema $ROOT_DIR/contracts-results/$1

INFO=$(cat <<-EOF
export CONTRACT_ADDRESS=${CONTRACT_ADDRESS}
export CODE_ID=${NEW_CODE_ID}
EOF
)
echo "$INFO" > "$ROOT_DIR/contracts-results/$1/info.env"
cd $ROOT_DIR
}

# Download the build-binary and setup-dev-network artifacts from cosmozone
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
tar -xf $BINARY_ARTIFACT_BIN
export PATH=$(pwd):$PATH

curl --output artifacts.zip --header "$TOKEN_TYPE: $TOKEN_VALUE" "https://gitlab-nomo.credissimo.net/api/v4/projects/3/jobs/artifacts/v$VERSION/download?job=setup-dev-network"
echo 'A' | unzip artifacts.zip

# Deploy or migrate contracts
CONTRACTS_VERSION=$(curl --header "$TOKEN_TYPE: $TOKEN_VALUE" "https://gitlab-nomo.credissimo.net/api/v4/projects/8/repository/tags" | jq '.[0].name' | tr -d '"')

  if [[ -d "last-contracts-version" ]]; then
      rm -rf last-contracts-version
  fi

mkdir last-contracts-version
cd last-contracts-version
curl --output contracts.zip --header "$TOKEN_TYPE: $TOKEN_VALUE" "https://gitlab-nomo.credissimo.net/api/v4/projects/8/jobs/artifacts/$CONTRACTS_VERSION/download?job=deploy:cargo"
echo 'A' | unzip contracts.zip
tar -xf $CONTRACTS_ARTIFACT_BIN
cd $ROOT_DIR


deployContract "oracle" ${ORACLE_MSG}
deployContract "borrow" ${BORROW_MSG}
deployContract "loan" ${LOAN_MSG}
# deployContract "treasury" ${TREASURY_MSG}
