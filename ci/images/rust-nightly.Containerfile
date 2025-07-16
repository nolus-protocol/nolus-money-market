ARG rust_id

FROM ${rust_id:?}

ARG nightly_version="2025-07-01"

RUN "rustup" "toolchain" "install" "nightly-${nightly_version:?}"

RUN "rustup" "default" "nightly-${nightly_version:?}"
