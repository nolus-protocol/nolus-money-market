#!/bin/bash
set -euxo pipefail


ROOT_DIR=$(pwd)

if [[ ! -e "contracts-schema" ]]; then
      mkdir "contracts-schema"
  fi

getContractSchema() {
  local contract_name="$1"

  if [[ -d "contracts-schema/$contract_name" ]]; then
      rm -rf contracts-schema/"$contract_name"
  fi

  cd contracts/"$contract_name"
  mkdir "$ROOT_DIR"/contracts-schema/"$contract_name"
  cp -R schema "$ROOT_DIR"/contracts-schema/"$contract_name"
  cd "$ROOT_DIR"
}

# Collect contract schema
getContractSchema "oracle"
getContractSchema "borrow"
getContractSchema "loan"
getContractSchema "treasury"
