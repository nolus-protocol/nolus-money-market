{
  writeShellScriptBin,
}:
writeShellScriptBin "for-workspaces" ''
  set -eu

  for workspace in "platform" "protocol" "tests" "tools"
  do
    (cd "./''${workspace}" && "''${@}")
  done
''
