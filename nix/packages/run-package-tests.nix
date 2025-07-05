{
  for-combinations,
  for-profiles,
  parse-package-name,
  run-tests,
  set-ci-env,
  writeShellScriptBin,
}:
writeShellScriptBin "run-package-tests" ''
  set -eu

  . "${parse-package-name}/bin/parse-package-name"

  "${set-ci-env}/bin/set-ci-env" \
    "${for-profiles}/bin/for-profiles" \
    "${for-combinations}/bin/for-combinations" \
    "${run-tests}/bin/run-tests" \
    --package "''${package}" \
    "''${@}"
''
