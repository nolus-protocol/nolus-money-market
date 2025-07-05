{
  cargo-each,
  dex,
  profile-mode,
  rust-stable,
  writeShellScriptBin,
}:
writeShellScriptBin "for-combinations" ''
  set -eu

  mode="$("${profile-mode}/bin/profile-mode")"
  readonly "mode"

  case "''${mode}" in
    ("ci" | "build") ;;
    (*)
      "echo" "Unrecognized mode: \"''${mode}\"!" >&2

      exit "1"
  esac

  run () {
    "${cargo-each}/bin/cargo-each" \
      "each" \
      "run" \
      --rust-path "${rust-stable}/bin" \
      --external-command \
      --tag "''${mode}" \
      "''${@}"
  }

  if dex="$("${dex}/bin/dex" 2>"/dev/null")"
  then
    "run" \
      --tag "''${dex}" \
      "''${@}"
  else
    "run" \
      "''${@}"
  fi
''
