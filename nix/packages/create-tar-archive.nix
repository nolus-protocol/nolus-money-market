{
  coreutils,
  findutils,
  gnutar,
  gnused,
  writeShellScriptBin,
}:
writeShellScriptBin "create-tar-archive" ''
  set -eu

  target="''${1:?"Target not set as first parameter!"}"
  shift

  target="''$("${coreutils}/bin/realpath" ''${target})"

  if "${coreutils}/bin/test" \
    "-d" \
    "''${target}"
  then
    directory="''${target}"

    target="."
  else
    directory="''$("${coreutils}/bin/dirname" "''${target}")"

    target="''$("${coreutils}/bin/basename" "''${target}")"
  fi

  readonly "directory"
  readonly "target"

  archive="''${1:?"Output archive name or path not set as second parameter!"}"
  shift

  archive="''$("${coreutils}/bin/realpath" "''${archive}")"
  readonly "archive"

  case "''${#}" in
    ("0") ;;
    (*)
      "echo" "Too many arguments!" >&2

      exit "1"
  esac

  (
    cd "''${directory}"

    files="''$(
      "${findutils}/bin/find" \
        "''${target}" \
        "!" \
        "(" \
        -path "." \
        -o \
        -path "./**/**" \
        ")"
    )"
    files="''$(
      "${gnused}/bin/sed" \
        -e "s/^\\.\\/\\(.\\+\\)\''$/\\1/g" \
        <<EOF
  ''${files:?}
  EOF
    )"
    readonly files

    "${findutils}/bin/xargs" \
      -E "" \
      -n "1" \
      "${gnutar}/bin/tar" \
      -r \
      -f "''${archive}" \
      <<EOF
  ''${files:?}
  EOF
  )
''
