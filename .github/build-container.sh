# This shell script is meant to be sourced with the POSIX shell "dot" command.

build() (
  image="${1:?"No image selected!"}"
  readonly "image"
  shift

  friendly_name="${1:?"No friendly name selected!"}"
  readonly "friendly_name"
  shift

  case "${1:?"Expected \"--\" to be passed!"}" in
    ("--")
      shift
      ;;
    (*)
      "echo" "Expected \"--\" to be passed!" >&2

      exit "1"
  esac

  "docker" \
    "buildx" \
    "build" \
    "${@:?}" \
    --file "./ci/images/${image:?}.Containerfile" \
    --iidfile "./.${friendly_name:?}-image-id" \
    "./ci/"
)
