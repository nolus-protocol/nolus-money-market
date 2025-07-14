{
  profile,
  profile-mode,
  writeShellScriptBin,
}:
writeShellScriptBin "for-profiles" ''
  set -eu

  mode="$("${profile-mode}/bin/profile-mode")"
  readonly "mode"

  case "''${mode}" in
    ("ci")
      for profile in "ci_dev" "ci_dev_no_debug_assertions"
      do
        "${profile}/bin/profile" "''${profile}" \
          "''${@}"
      done
      ;;
    ("build")
      "echo" "Unsupported mode: \"''${mode}\"!" >&2

      "exit" "1"
      ;;
    (*)
      "echo" "Unrecognized mode: \"''${mode}\"!" >&2

      "exit" "1"
  esac
''
