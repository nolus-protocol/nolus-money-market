{
  coreutils,
  for-combinations,
  for-workspaces,
  profile,
  run-tests,
  set-ci-env,
  writeShellScriptBin,
}:
writeShellScriptBin "ci-run-tests" ''
  set -eu

  profile="''${1?"Profile must be passed as a first pameter!"}"
  readonly "profile"

  target_dir="$("${coreutils}/bin/realpath" "./target/")"
  readonly "target_dir"

  "${set-ci-env}/bin/set-ci-env" \
    "${profile}/bin/profile" "''${profile}" \
    "${for-workspaces}/bin/for-workspaces" \
    "${for-combinations}/bin/for-combinations" \
    "${run-tests}/bin/run-tests" \
    "''${@}" \
    "--" \
    "--target-dir" "''${target_dir}"
''
