{
  for-profiles,
  jq,
  lib,
  profile,
  set-ci-env,
  writeShellScriptBin,
}:
let
  to-json = writeShellScriptBin "to-json" ''
    set -eu

    profile="$("profile")"

    "echo" "''${profile}" | \
      "jq" \
        --raw-input
  '';

  path = lib.makeBinPath [
    for-profiles
    jq
    profile
    to-json
    set-ci-env
  ];
in
writeShellScriptBin "ci-profiles-json" ''
  set -eu

  PATH="${path}"
  readonly "PATH"
  export "PATH"

  profiles="$(
    "set-ci-env" \
      "for-profiles" \
      "to-json"
  )"

  echo "''${profiles}" | \
    "jq" \
      --compact-output \
      --slurp
''
