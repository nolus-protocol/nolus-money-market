{
  check-lockfiles,
  for-workspaces,
  writeShellScriptBin,
}:
writeShellScriptBin "ci-check-lockfiles" ''
  "${for-workspaces}/bin/for-workspaces" \
    "${check-lockfiles}/bin/check-lockfiles"
''
