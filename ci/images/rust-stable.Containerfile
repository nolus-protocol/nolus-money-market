ARG stable_version="1.88-alpine"

FROM docker.io/library/rust:${stable_version:?} AS local-rust-stable

VOLUME ["/src"]

WORKDIR "/src"

ENV CARGO_TARGET_DIR="/build/"

RUN ["apk", "update"]

RUN ["apk", "add", "libc-dev"]
