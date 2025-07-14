{
  check-unused-deps,
  for-combinations,
  parse-package-name,
  profile,
  set-ci-env,
  writeShellScriptBin,
}:
writeShellScriptBin "check-package-unused-deps" ''
  set -eu

  . "${parse-package-name}/bin/parse-package-name"

  "${set-ci-env}/bin/set-ci-env" \
    "${profile}/bin/profile" "dev" \
    "${for-combinations}/bin/for-combinations" \
    "${check-unused-deps}/bin/check-unused-deps" \
    --package "''${package}" \
    "''${@}"
''
