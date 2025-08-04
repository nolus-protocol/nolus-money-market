# syntax=docker/dockerfile:1

################################################################################
##                         START : EDIT  HERE : START                         ##
################################################################################

ARG cargo_audit_ver="0.21.2"

ARG cargo_udeps_ver="0.1.57"

ARG cosmwasm_capabilities="cosmwasm_1_1,cosmwasm_1_2,iterator,neutron,staking,\
stargate"

ARG platform_contracts_count="3"

ARG production_network_build_profile="production_nets_release"

ARG production_network_max_binary_size="5M"

ARG protocol_contracts_count="7"

ARG rust_image_ver="1.88"

### 1.90
ARG rust_nightly_ver="2025-08-01"

ARG test_network_build_profile="test_nets_release"

ARG test_network_max_binary_size="5M"

################################################################################
##                           END : EDIT  HERE : END                           ##
################################################################################

FROM docker.io/library/rust:${rust_image_ver:?}-alpine AS rust

ENV SOURCE_DATE_EPOCH="0"

VOLUME ["/src"]

WORKDIR "/src/"

ENV CARGO_TARGET_DIR="/tmp/cargo-target/"

ENV CARGO_TERM_COLOR="always"

ENV POSIXLY_CORRECT="1"

RUN ["apk", "update"]

RUN ["apk", "add", "libc-dev"]

FROM rust AS cargo-audit

ARG cargo_audit_ver

RUN \
  --mount=type="tmpfs",target="/tmp/cargo-target/" \
  "cargo" "install" "cargo-audit@${cargo_audit_ver:?}"

FROM rust AS cargo-each

RUN \
  --mount=type="bind",from="tools",target="/src/",readonly \
  --mount=type="tmpfs",target="/tmp/cargo-target/" \
  ["cargo", "install", "--path", "/src/cargo-each/"]

FROM rust AS cargo-udeps

RUN ["apk", "add", "ca-certificates", "openssl-dev", "openssl-libs-static"]

ARG cargo_udeps_ver

RUN \
  --mount=type="tmpfs",target="/tmp/cargo-target/" \
  "cargo" "install" "cargo-udeps@${cargo_udeps_ver:?}"

FROM rust AS rust-ci

ENV SOFTWARE_RELEASE_ID="ci-software-release"

ENV PROTOCOL_NETWORK="ci-network"

ENV PROTOCOL_NAME="ci-protocol"

ENV PROTOCOL_RELEASE_ID="ci-protocol-release"

FROM rust-ci AS rust-ci-multi-workspace

COPY \
  --chmod="0555" \
  "./scripts/for-each-workspace.sh" \
  "/usr/local/bin/"

FROM rust-ci-multi-workspace AS audit-dependencies

COPY \
  --from=cargo-audit \
  "/usr/local/cargo/bin/cargo-audit" \
  "/usr/local/bin/"

ENTRYPOINT ["/usr/local/bin/for-each-workspace.sh", "cargo", "audit"]

FROM rust-ci-multi-workspace AS check-formatting

RUN ["rustup", "component", "add", "rustfmt"]

ENTRYPOINT ["/usr/local/bin/for-each-workspace.sh", "cargo", "fmt", "--check"]

FROM rust-ci-multi-workspace AS check-lockfiles

COPY \
  --chmod="0555" \
  "./scripts/check-lockfiles.sh" \
  "/usr/local/bin/"

ENTRYPOINT ["/usr/local/bin/for-each-workspace.sh", "check-lockfiles.sh"]

FROM rust-ci AS check-unused-dependencies

RUN ["apk", "add", "util-linux"]

ARG rust_nightly_ver

ENV RUST_NIGHTLY_VERSION="nightly-${rust_nightly_ver:?}"

RUN "rustup" "toolchain" "install" "${RUST_NIGHTLY_VERSION:?}"

COPY \
  --from=cargo-each \
  "/usr/local/cargo/bin/cargo-each" \
  "/usr/local/bin/"

COPY \
  --from=cargo-udeps \
  "/usr/local/cargo/bin/cargo-udeps" \
  "/usr/local/bin/"

COPY \
  --chmod="0555" \
  "./scripts/check-unused-deps.sh" \
  "/usr/local/bin/"

ENTRYPOINT ["/usr/local/bin/check-unused-deps.sh"]

FROM rust-ci AS lint

RUN ["apk", "add", "util-linux"]

RUN ["rustup", "component", "add", "clippy"]

COPY \
  --from=cargo-each \
  "/usr/local/cargo/bin/cargo-each" \
  "/usr/local/bin/"

COPY \
  --chmod="0555" \
  "./scripts/lint.internal.sh" \
  "/usr/local/bin/"

COPY \
  --chmod="0555" \
  "./scripts/lint.sh" \
  "/usr/local/bin/"

ENTRYPOINT ["/usr/local/bin/lint.sh"]

FROM rust-ci AS test

RUN ["apk", "add", "util-linux"]

COPY \
  --from=cargo-each \
  "/usr/local/cargo/bin/cargo-each" \
  "/usr/local/bin/"

COPY \
  --chmod="0555" \
  "./scripts/test.sh" \
  "/usr/local/bin/"

ENTRYPOINT ["/usr/local/bin/test.sh"]

FROM rust AS build

VOLUME ["/artifacts"]

RUN ["apk", "add", "util-linux"]

COPY \
  --from=cargo-each \
  "/usr/local/cargo/bin/cargo-each" \
  "/usr/local/bin/"
