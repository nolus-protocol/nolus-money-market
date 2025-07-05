{
  for-combinations,
  for-profiles,
  lint,
  parse-package-name,
  writeShellScriptBin,
}:
writeShellScriptBin "lint-package" ''
  set -eu

  . "${parse-package-name}/bin/parse-package-name"

  "${for-profiles}/bin/for-profiles" \
    "${for-combinations}/bin/for-combinations" \
    "${lint}/bin/lint" \
    --package "''${package}" \
    "''${@}"
''
