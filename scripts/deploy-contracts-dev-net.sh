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
CONTRACTS_ARTIFACT_BIN="contracts.tar.gz"
NOLUS_DEV_NET="https://net-dev.nolus.io:26612"
GITLAB_API="https://gitlab-nomo.credissimo.net/api/v4"
ACCOUNTS_DIR="$(pwd)/accounts"
export TXFLAG="--gas-prices 0.025unolus --gas auto --gas-adjustment 1.3 -y --home $ACCOUNTS_DIR --node $NOLUS_DEV_NET"

source msgs.env

downloadArtifact() {
  curl --output $1.zip --header "$TOKEN_TYPE: $TOKEN_VALUE" "$GITLAB_API/projects/3/jobs/artifacts/v$2/download?job=$1"
  echo 'A' | unzip $1.zip
}

deployContract() {
  RES=$(nolusd tx wasm store artifacts/$1.wasm --from treasury ${TXFLAG} --output json -b block)
  NEW_CODE_ID=$(echo $RES | jq -r '.logs[0].events[-1].attributes[0].value')

  nolusd tx wasm instantiate $NEW_CODE_ID "$2" --from treasury --label "$1" ${TXFLAG} --no-admin
  sleep 6
  CONTRACT_ADDRESS=$(nolusd query wasm list-contract-by-code $NEW_CODE_ID --node $NOLUS_DEV_NET --output json | jq -r '.contracts[-1]')

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
VERSION=$(curl --silent "$NOLUS_DEV_NET/abci_info" | jq '.result.response.version' | tr -d '"')

downloadArtifact "setup-dev-network" $VERSION
downloadArtifact "build-binary" $VERSION
tar -xf $BINARY_ARTIFACT_BIN
export PATH=$(pwd):$PATH

# Deploy smart contracts
deployContract "oracle" ${ORACLE_INIT_MSG}
deployContract "borrow" ${BORROW_INIT_MSG}
deployContract "loan" ${LOAN_INIT_MSG}
deployContract "treasury" ${TREASURY_INIT_MSG}
