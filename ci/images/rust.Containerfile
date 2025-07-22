ARG stable_version="1.88-alpine"

FROM docker.io/library/rust:${stable_version:?} AS local-rust-stable

VOLUME ["/src"]

WORKDIR "/src"

ENV CARGO_TARGET_DIR="/build/"

RUN ["apk", "update"]

RUN ["apk", "add", "ca-certificates", "libc-dev", "openssl-dev", "openssl-libs-static"]

RUN ["rustup", "component", "add", "clippy", "rustfmt"]

ENV RUST_NIGHTLY_VERSION="nightly-2025-07-01"

RUN "rustup" "toolchain" "install" "${RUST_NIGHTLY_VERSION:?}"
