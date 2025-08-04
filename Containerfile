# syntax=docker/dockerfile:1

################################################################################
##                         START : EDIT  HERE : START                         ##
################################################################################

ARG binaryen_checksum="e959f2170af4c20c552e9de3a0253704d6a9d2766e8fdb88e4d6ac4bae9388fe"

ARG binaryen_version="123"

ARG cargo_audit_ver="0.21.2"

ARG cargo_udeps_ver="0.1.57"

ARG cosmwasm_check_ver="3.0.1"

ARG cosmwasm_capabilities="cosmwasm_1_1,cosmwasm_1_2,iterator,neutron,staking,stargate"

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

WORKDIR "/src"

ENV CARGO_TARGET_DIR="/tmp/cargo-target/"

ENV CARGO_TERM_COLOR="always"

ENV POSIXLY_CORRECT="1"

RUN <<EOF
"apk" "update"
"apk" "add" "libc-dev"
EOF

FROM rust AS cargo-audit

ARG cargo_audit_ver

RUN \
  --mount=type="tmpfs",target="/tmp/cargo-target/" \
  "cargo" "install" "cargo-audit@${cargo_audit_ver:?}"

FROM rust AS cargo-each

RUN \
  --mount=type="bind",from="tools",target="/src/",readonly \
  --mount=type="tmpfs",target="/tmp/cargo-target/" \
  "cargo" "install" "--path" "/src/cargo-each/"

FROM rust AS cargo-udeps

RUN "apk" "add" "ca-certificates" "openssl-dev" "openssl-libs-static"

ARG cargo_udeps_ver

RUN \
  --mount=type="tmpfs",target="/tmp/cargo-target/" \
  "cargo" "install" "cargo-udeps@${cargo_udeps_ver:?}"

FROM docker.io/library/rust:${rust_image_ver:?}-bookworm AS cosmwasm-check

ARG cosmwasm_check_ver

RUN \
  --mount=type="tmpfs",target="/tmp/cargo-target/" \
  "cargo" "install" "cosmwasm-check@${cosmwasm_check_ver:?}" "--target-dir" "/tmp/cargo-target"

FROM rust AS rust-ci

VOLUME ["/src"]

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

ENTRYPOINT ["/usr/local/bin/for-each-workspace.sh", "cargo", "audit"]

COPY \
  --from=cargo-audit \
  "/usr/local/cargo/bin/cargo-audit" \
  "/usr/local/bin/"
  # --link=true \

FROM rust-ci-multi-workspace AS check-formatting

RUN "rustup" "component" "add" "rustfmt"

ENTRYPOINT ["/usr/local/bin/for-each-workspace.sh", "cargo", "fmt", "--check"]

FROM rust-ci-multi-workspace AS check-lockfiles

ENTRYPOINT ["/usr/local/bin/for-each-workspace.sh", "check-lockfiles.sh"]

COPY \
  --chmod="0555" \
  "./scripts/check-lockfiles.sh" \
  "/usr/local/bin/"
  # --link=true \

FROM rust-ci AS check-unused-dependencies

RUN "apk" "add" "util-linux"

ARG rust_nightly_ver

ENV RUST_NIGHTLY_VERSION="nightly-${rust_nightly_ver:?}"

RUN "rustup" "toolchain" "install" "${RUST_NIGHTLY_VERSION:?}"

ENTRYPOINT ["/usr/local/bin/check-unused-deps.sh"]

COPY \
  --from=cargo-udeps \
  "/usr/local/cargo/bin/cargo-udeps" \
  "/usr/local/bin/"
  # --link=true \

COPY \
  --chmod="0555" \
  "./scripts/check-unused-deps.sh" \
  "/usr/local/bin/"
  # --link=true \

COPY \
  --from=cargo-each \
  "/usr/local/cargo/bin/cargo-each" \
  "/usr/local/bin/"
  # --link=true \

FROM rust-ci AS lint

RUN <<EOF
"apk" "add" "util-linux"
"rustup" "component" "add" "clippy"
EOF

ENTRYPOINT ["/usr/local/bin/lint.sh"]

COPY \
  --chmod="0555" \
  "./scripts/lint.sh" \
  "./scripts/lint.workspace.sh" \
  "/usr/local/bin/"
  # --link=true \

COPY \
  --from=cargo-each \
  "/usr/local/cargo/bin/cargo-each" \
  "/usr/local/bin/"
  # --link=true \
  
FROM rust-ci AS test

RUN "apk" "add" "util-linux"

ENTRYPOINT ["/usr/local/bin/test.sh"]

COPY \
  --chmod="0555" \
  "./scripts/test.sh" \
  "/usr/local/bin/"
  # --link=true \

COPY \
  --from=cargo-each \
  "/usr/local/cargo/bin/cargo-each" \
  "/usr/local/bin/"
  # --link=true \

FROM docker.io/library/busybox:latest AS binaryen

ARG binaryen_checksum

ARG binaryen_version

ADD \
  --checksum=sha256:${binaryen_checksum:?} \
  "https://github.com/WebAssembly/binaryen/releases/download/version_${binaryen_version:?}/binaryen-version_${binaryen_version:?}-x86_64-linux.tar.gz" \
  "/binaryen.tar.gz"
  # --link=true \

RUN <<EOF
cd "/"

"tar" \
  "x" \
  -f "/binaryen.tar.gz" \
  "binaryen-version_${binaryen_version:?}/bin/wasm-opt"

"mv" \
  "/binaryen-version_${binaryen_version:?}/bin/wasm-opt" \
  "/"

"rm" \
  -fr \
  "/binaryen-version_${binaryen_version:?}"
EOF

FROM rust AS build

VOLUME ["/artifacts"]

RUN <<EOF
"apk" "add" "gcompat" "util-linux"
"rustup" "target" "add" "wasm32-unknown-unknown"
EOF

ARG cosmwasm_capabilities

ENV COSMWASM_CAPABILITIES="${cosmwasm_capabilities:?}"

ARG platform_contracts_count

ENV PLATFORM_CONTRACTS_COUNT="${platform_contracts_count:?}"

ARG production_network_build_profile

ENV PRODUCTION_NETWORK_BUILD_PROFILE="${production_network_build_profile:?}"

ARG production_network_max_binary_size

ENV PRODUCTION_NETWORK_MAX_BINARY_SIZE="${production_network_max_binary_size:?}"

ARG protocol_contracts_count

ENV PROTOCOL_CONTRACTS_COUNT="${protocol_contracts_count:?}"

ARG test_network_build_profile

ENV TEST_NETWORK_BUILD_PROFILE="${test_network_build_profile:?}"

ARG test_network_max_binary_size

ENV TEST_NETWORK_MAX_BINARY_SIZE="${test_network_max_binary_size:?}"

COPY \
  --chmod="0555" \
  "./scripts/build.sh" \
  "/usr/local/bin/"
  # --link=true \

ARG binaryen_version

ENV BINARYEN_VERSION="${binaryen_version:?}"

COPY \
  --from=binaryen \
  "/wasm-opt" \
  "/usr/local/bin/"
  # --link=true \

COPY \
  --from=cosmwasm-check \
  "/usr/local/cargo/bin/cosmwasm-check" \
  "/usr/local/bin/"
  # --link=true \

COPY \
  --from=cargo-each \
  "/usr/local/cargo/bin/cargo-each" \
  "/usr/local/bin/"
  # --link=true \

COPY \
  --from="src" \
  "." \
  "/src"

FROM build AS platform

WORKDIR "/src/platform"

ENTRYPOINT ["/usr/local/bin/build.sh", "platform"]

FROM build AS protocol

WORKDIR "/src/protocol"

ENTRYPOINT ["/usr/local/bin/build.sh", "protocol"]
