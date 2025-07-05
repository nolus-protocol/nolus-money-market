{
  profile-mode,
  writeShellScriptBin,
}:
writeShellScriptBin "set-ci-env" ''
  SOFTWARE_RELEASE_ID="ci" \
    PROTOCOL_NAME="ci" \
    PROTOCOL_NETWORK="ci" \
    PROTOCOL_RELEASE_ID="ci" \
    "${profile-mode}/bin/profile-mode" "ci" \
    "''${@}"
''
