ARG rust_id

FROM ${rust_id:?}

RUN ["rustup", "component", "add", "rustfmt"]

COPY \
  --chmod="0555" \
  "./scripts/for-each-workspace.sh" \
  "/bin/"

ENTRYPOINT ["/bin/for-each-workspace.sh", "cargo", "fmt", "--check"]
