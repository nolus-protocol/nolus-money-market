################################################################################
##                         START : EDIT  HERE : START                         ##
################################################################################

ARG rust_ver="1.85-slim"

################################################################################
##                           END : EDIT  HERE : END                           ##
################################################################################

FROM docker.io/library/rust:${rust_ver:?} AS builder

USER 0:0

RUN ["mkdir", "-m", "0775", "/code/"]

RUN ["mkdir", "-m", "01557", "/build/"]

RUN ["chmod", "01777", "/tmp/"]

RUN ["chmod", "-R", "01557", "/usr/local/cargo/"]

ENV CARGO_TARGET_DIR="/build/target/"

RUN ["apt", "update"]

RUN ["apt", "upgrade", "--yes"]

RUN ["apt", "install", "--yes", "gcc", "libssl-dev", "pkg-config"]

USER 1000:1000

FROM builder AS cargo-audit

RUN ["cargo", "install", "--jobs", "1", "--force", "cargo-audit"]

FROM builder AS cargo-each

RUN \
    --mount=type=bind,source="./tools/",target="/code/" \
    [ \
        "cargo", \
        "install", \
        "--jobs", "1", \
        "--force", \
        "--path", "/code/cargo-each/" \
    ]

FROM builder AS cargo-udeps

RUN ["cargo", "install", "--jobs", "1", "--force", "cargo-udeps"]

FROM builder

USER 0:0

RUN ["rustup", "component", "add", "clippy", "rustfmt"]

RUN ["rustup", "target", "add", "wasm32-unknown-unknown"]

RUN ["rustup", "toolchain", "add", "nightly"]

USER 1000:1000

ENTRYPOINT ["/check.sh"]

COPY \
    --chmod="0555" \
    --chown=0:0 \
    --from=cargo-audit \
    "/usr/local/cargo/bin/" \
    "/usr/local/cargo/bin/"

COPY \
    --chmod="0555" \
    --chown=0:0 \
    --from=cargo-each \
    "/usr/local/cargo/bin/" \
    "/usr/local/cargo/bin/"

COPY \
    --chmod="0555" \
    --chown=0:0 \
    --from=cargo-udeps \
    "/usr/local/cargo/bin/" \
    "/usr/local/cargo/bin/"

COPY \
    --chmod="0555" \
    --chown=0:0 \
    "./scripts/check/*.sh" \
    "/"

USER 0:0

VOLUME ["/code/"]

RUN ["chmod", "-R", "0555", "/code/"]

USER 1000:1000
