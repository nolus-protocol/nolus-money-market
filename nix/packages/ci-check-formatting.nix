{
  check-formatting,
  for-workspaces,
  writeShellScriptBin,
}:
writeShellScriptBin "ci-check-formatting" ''
  "${for-workspaces}/bin/for-workspaces" \
    "${check-formatting}/bin/check-formatting"
''
