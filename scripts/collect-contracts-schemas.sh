#!/bin/bash
set -euxo pipefail

ROOT_DIR=$(pwd)
CONTRACTS_SCHEMA_DIR="contracts-schemas"

if [[ ! -e "$CONTRACTS_SCHEMA_DIR" ]]; then
      mkdir "$CONTRACTS_SCHEMA_DIR"
fi

copyContractSchema() {
  local contract_name="$1"

  if [[ -d "$CONTRACTS_SCHEMA_DIR/$contract_name" ]]; then
      rm -rf "${CONTRACTS_SCHEMA_DIR:?}/$contract_name"
  fi

  mkdir "$ROOT_DIR"/"$CONTRACTS_SCHEMA_DIR"/"$contract_name"
  cp -R contracts/"$contract_name"/schema "$ROOT_DIR"/"$CONTRACTS_SCHEMA_DIR"/"$contract_name"
}

# Collect contracts schemas
copyContractSchema "oracle"
copyContractSchema "leaser"
copyContractSchema "lease"
copyContractSchema "treasury"
