ARG rust_ci_id

FROM ${rust_ci_id:?}

COPY \
  --chmod="0555" \
  "./scripts/lint.sh" \
  "/bin/lint.sh"

COPY \
  --chmod="0555" \
  "./scripts/lint.internal.sh" \
  "/bin/"

ENTRYPOINT ["/bin/lint.sh"]
