ARG rust_ci_id

FROM ${rust_ci_id:?}

COPY \
  --chmod="0555" \
  "./scripts/test.sh" \
  "/bin/test.sh"

ENTRYPOINT ["/bin/test.sh"]
