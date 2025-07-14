{
  check-unused-deps,
  coreutils,
  for-combinations,
  for-workspaces,
  set-ci-env,
  writeShellScriptBin,
}:
writeShellScriptBin "ci-check-unused-deps" ''
  set -eu

  target_dir="$("${coreutils}/bin/realpath" "./target/")"
  readonly "target_dir"

  "${set-ci-env}/bin/set-ci-env" \
    "${for-workspaces}/bin/for-workspaces" \
    "${for-combinations}/bin/for-combinations" \
    "${check-unused-deps}/bin/check-unused-deps" \
    "''${@}" \
    "--" \
    "--target-dir" "''${target_dir}"
''
