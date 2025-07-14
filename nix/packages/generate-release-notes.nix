{
  git,
  git-cliff,
  lib,
  writeShellScriptBin,
}:
let
  path = lib.makeBinPath [
    git
    git-cliff
  ];
in
writeShellScriptBin "generate-release-notes" ''
  PATH="${path}" \
    "git" \
      "cliff" \
      --current \
      "''${@}"
''
