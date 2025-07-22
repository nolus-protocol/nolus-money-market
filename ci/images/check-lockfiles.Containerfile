ARG rust_id

FROM ${rust_id:?}

COPY \
  --chmod="0555" \
  "./scripts/for-each-workspace.sh" \
  "/bin/"

COPY \
  --chmod="0555" \
  "./scripts/check-lockfiles.sh" \
  "/bin/"

ENTRYPOINT ["/bin/for-each-workspace.sh", "/bin/check-lockfiles.sh"]
