ARG rust_id

FROM ${rust_id:?} AS internal-cargo-each-builder

RUN \
  --mount=type="bind",from="tools",target="/src/",readonly \
  --mount=type="tmpfs",target="/tmp/cargo-target/" \
  ["cargo", "install", "--path", "/src/cargo-each"]

FROM scratch

COPY \
  --from="internal-cargo-each-builder" \
  "/usr/local/cargo/bin/cargo-each" \
  "/bin/cargo-each"
