ARG cargo_udeps_id

ARG rust_ci_id

FROM ${cargo_udeps_id:?} AS cargo-udeps

FROM ${rust_ci_id:?}

COPY \
  --from=cargo-udeps \
  "/bin/cargo-udeps" \
  "/bin/"

COPY \
  --chmod="0555" \
  "./scripts/check-unused-deps.sh" \
  "/bin/"

ENTRYPOINT ["/bin/check-unused-deps.sh"]
