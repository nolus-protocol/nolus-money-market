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
writeShellScriptBin "check-formatting" ''
  PATH="${path}" \
    "cargo" \
    "fmt" \
    "''${@}" \
    --check
''
