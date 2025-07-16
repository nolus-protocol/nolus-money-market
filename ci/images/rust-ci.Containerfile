ARG cargo_each_id

ARG rust_id

FROM ${cargo_each_id:?} AS cargo-each

FROM ${rust_id:?}

ENV SOFTWARE_RELEASE_ID="ci-software-release"

ENV PROTOCOL_NETWORK="ci-network"

ENV PROTOCOL_NAME="ci-protocol"

ENV PROTOCOL_RELEASE_ID="ci-protocol-release"

COPY \
  --from=cargo-each \
  "/bin/cargo-each" \
  "/bin/"

COPY \
  --chmod="0555" \
  "./scripts/for-each-workspace.sh" \
  "/bin/"
