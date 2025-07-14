{
  writeShellScriptBin,
}:
writeShellScriptBin "parse-package-name" ''
  package="''${1?"Package not set as first parameter!"}"
  readonly "package"
  shift

  case "''${#}" in
    ("0") ;;
    (*)
      case "''${1}" in
        ("--")
          shift
          ;;
        (*)
          "echo" "Expected \"--\" after package's name! Got: \"''${1}\"!" >&2

          exit "1"
      esac
  esac
''
