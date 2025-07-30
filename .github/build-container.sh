# This shell script is meant to be sourced with the POSIX shell "dot" command.

build() (
  set -eu

  image="${1:?"No image selected!"}"
  readonly "image"
  shift

  case "${#:?}" in
    ("0") ;;
    (*)
      case "${1:?"Expected \"--\" to be passed!"}" in
        ("--")
          shift
          ;;
        (*)
          "echo" "Expected \"--\" to be passed!" >&2

          exit "1"
      esac
  esac

  SOURCE_DATE_EPOCH="0" \
    "docker" \
      "buildx" \
      "build" \
      --build-arg "SOURCE_DATE_EPOCH=0" \
      --cache-to "type=inline" \
      --file "./ci/images/${image:?}.Containerfile" \
      --load \
      --provenance "false" \
      --quiet \
      --tag "localhost/local/${image:?}:${runner_name:?}-${hash:?}" \
      "${@}" \
      "./ci/" \
      >"./.${image:?}-image-id"

  "docker" \
    "image" \
    "tag" \
    "localhost/local/${image:?}:${runner_name:?}-${hash:?}" \
    "localhost/local/${image:?}:${hash:?}"
)
