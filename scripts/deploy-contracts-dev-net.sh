#!/bin/bash
set -euxo pipefail

if [[ -z ${CI_JOB_TOKEN+x} ]]; then
  echo "Error: there is no CI_JOB token"
  exit 1
fi

TOKEN_TYPE="JOB-TOKEN"
TOKEN_VALUE="$CI_JOB_TOKEN"

COMMON_DIR="$(pwd)/scripts/common"

BINARY_ARTIFACT_BIN="nolus.tar.gz"
NOLUS_DEV_NET="https://net-dev.nolus.io:26612"
GITLAB_API="https://gitlab-nomo.credissimo.net/api/v4"
COSMZONE_PROJECT_ID="3"
SETUP_DEV_NETWORK_ARTIFACT="setup-dev-network"
NOLUS_BUILD_BINARY_ARTIFACT="build-binary"
STABLE_DENOM="$STABLE_DENOM_DEV"
HOME_DIR="$(pwd)/accounts"
CONTRACTS_RESULTS_FILE="$1"

downloadArtifact() {
  local name="$1"
  local version="$2"
  local project_id="$3"

  curl --output "$name".zip --header "$TOKEN_TYPE: $TOKEN_VALUE" "$GITLAB_API/projects/$project_id/jobs/artifacts/v$version/download?job=$name"
  echo 'A' | unzip "$name".zip
}

# Download the build-binary and setup-dev-network artifacts from cosmozone

VERSION=$(curl --silent "$NOLUS_DEV_NET/abci_info" | jq '.result.response.version' | tr -d '"')
downloadArtifact "$SETUP_DEV_NETWORK_ARTIFACT" "$VERSION" "$COSMZONE_PROJECT_ID"
downloadArtifact "$NOLUS_BUILD_BINARY_ARTIFACT" "$VERSION" "$COSMZONE_PROJECT_ID"
tar -xf $BINARY_ARTIFACT_BIN

export PATH;
PATH=$(pwd):$PATH

# Deploy contracts

source "$COMMON_DIR"/deploy-contracts.sh
deployContracts "$CONTRACTS_RESULTS_FILE" "$NOLUS_DEV_NET" "$HOME_DIR" "$STABLE_DENOM"

