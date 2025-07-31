# This shell script is meant to be sourced with the POSIX shell "dot" command.

set -x

___get_image() {
  set -eu

  image="${1:?"No image selected!"}"

  case "${#:?}" in
    ("1")
      shift_by="1"
      ;;
    (*)
      case "${2:?"Expected \"--\" to be passed!"}" in
        ("--")
          shift_by="2"
          ;;
        (*)
          "echo" \
            "Expected \"--\" to be passed!" \
            >&2

          exit "1"
      esac
  esac
}

build() (
  "___get_image" "${@}"
  shift "${shift_by:?}"
  unset "shift_by"

  id="$(
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
        "${@}" \
        "./ci/"
  )"

  "echo" \
    "${id:?}" \
    >"./.${image:?}-image-id"

  "docker" \
    "image" \
    "ls" \
    --no-trunc

  "docker" \
    "image" \
    "tag" \
    "${id:?}" \
    "localhost/local/${image:?}:${hash:?}"
)

build_into_dir() (
  "___get_image" "${@}"
  shift "${shift_by:?}"
  unset "shift_by"

  "build" \
    "${image:?}" \
    -- \
    --output "type=tar,dest=./.${image:?}-image.tar" \
    "${@}"

  readonly "image"

  "mkdir" "./.${image:?}-image"

  (
    cd "./.${image:?}-image"

    "tar" "fx" "../.${image:?}-image.tar"
  )
)
