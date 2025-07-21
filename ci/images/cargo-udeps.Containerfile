ARG cargo_udeps_version="0.1.57"

ARG rust_id

FROM ${rust_id:?} AS internal-cargo-udeps-builder

ARG cargo_udeps_version

RUN "cargo" "install" "cargo-udeps@${cargo_udeps_version:?}"

FROM scratch

COPY \
  --from="internal-cargo-udeps-builder" \
  "/usr/local/cargo/bin/cargo-udeps" \
  "/bin/cargo-udeps"
