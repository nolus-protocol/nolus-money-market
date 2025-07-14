{
  lib,
  rust-stable,
  writeShellScriptBin,
}:
let
  path = lib.makeBinPath [
    rust-stable
  ];
in
writeShellScriptBin "check-lockfiles" ''
  PATH="${path}" \
    "cargo" \
    "update" \
    --locked \
    "''${@}"
''
