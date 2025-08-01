# syntax=docker/dockerfile:1

ARG cargo_audit_ver="0.21.2"

ARG cargo_udeps_ver="0.1.57"

### 1.88-alpine
ARG rust_digest="sha256:9dfaae478ecd298b6b5a039e1f2cc4fc040fc818a2de9aa78fa714dea036574d"

### 1.90
ARG rust_nightly_ver="2025-08-01"

ARG SOURCE_DATE_EPOCH="0"

FROM docker.io/library/rust@${rust_digest:?} AS rust

ARG SOURCE_DATE_EPOCH

VOLUME ["/src"]

WORKDIR "/src/"

ENV CARGO_TARGET_DIR="/build/"

ENV CARGO_TERM_COLOR="always"

ENV POSIXLY_CORRECT="1"

RUN ["apk", "update"]

RUN ["apk", "add", "libc-dev"]

FROM rust AS cargo-audit

ARG SOURCE_DATE_EPOCH

RUN ["apk", "add", "ca-certificates", "openssl-dev", "openssl-libs-static"]

ARG cargo_audit_ver

RUN \
  --mount=type="tmpfs",target="/tmp/cargo-target/" \
  "cargo" "install" "cargo-audit@${cargo_audit_ver:?}" "--target-dir" "/tmp/cargo-target/"

FROM rust AS cargo-each

ARG SOURCE_DATE_EPOCH

RUN \
  --mount=type="bind",from="tools",target="/src/",readonly \
  --mount=type="tmpfs",target="/tmp/cargo-target/" \
  ["cargo", "install", "--path", "/src/cargo-each/", "--target-dir", "/tmp/cargo-target/"]

FROM rust AS cargo-udeps

ARG SOURCE_DATE_EPOCH

ARG cargo_udeps_ver

RUN \
  --mount=type="tmpfs",target="/tmp/cargo-target/" \
  "cargo" "install" "cargo-udeps@${cargo_udeps_ver:?}" "--target-dir" "/tmp/cargo-target/"

FROM rust AS rust-nightly

ARG SOURCE_DATE_EPOCH

ARG rust_nightly_ver

ENV RUST_NIGHTLY_VERSION="nightly-${rust_nightly_ver:?}"

RUN "rustup" "toolchain" "install" "${RUST_NIGHTLY_VERSION:?}"

FROM rust AS rust-ci

ARG SOURCE_DATE_EPOCH

ENV SOFTWARE_RELEASE_ID="ci-software-release"

ENV PROTOCOL_NETWORK="ci-network"

ENV PROTOCOL_NAME="ci-protocol"

ENV PROTOCOL_RELEASE_ID="ci-protocol-release"

FROM rust-ci AS rust-ci-multi-workspace

ARG SOURCE_DATE_EPOCH

COPY \
  --chmod="0555" \
  "./scripts/for-each-workspace.sh" \
  "/usr/local/bin/"

FROM rust-ci-multi-workspace AS audit-dependencies

ARG SOURCE_DATE_EPOCH

COPY \
  --from=cargo-audit \
  "/usr/local/cargo/bin/cargo-audit" \
  "/usr/local/bin/"

ENTRYPOINT ["/usr/local/bin/for-each-workspace.sh", "cargo", "audit"]

FROM rust-ci-multi-workspace AS check-formatting

ARG SOURCE_DATE_EPOCH

RUN ["rustup", "component", "add", "rustfmt"]

ENTRYPOINT ["/usr/local/bin/for-each-workspace.sh", "cargo", "fmt", "--check"]

FROM rust-ci-multi-workspace AS check-lockfiles

ARG SOURCE_DATE_EPOCH

COPY \
  --chmod="0555" \
  "./scripts/check-lockfiles.sh" \
  "/usr/local/bin/"

ENTRYPOINT ["/usr/local/bin/for-each-workspace.sh", "check-lockfiles.sh"]

FROM rust-ci AS check-unused-dependencies

ARG SOURCE_DATE_EPOCH

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

ARG SOURCE_DATE_EPOCH

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

ARG SOURCE_DATE_EPOCH

RUN ["rustup", "component", "add", "clippy"]

COPY \
  --from=cargo-each \
  "/usr/local/cargo/bin/cargo-each" \
  "/usr/local/bin/"

COPY \
  --chmod="0555" \
  "./scripts/test.sh" \
  "/usr/local/bin/"

ENTRYPOINT ["/usr/local/bin/test.sh"]
