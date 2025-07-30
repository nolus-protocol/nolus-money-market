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
      --file "./ci/images/${image:?}.Containerfile" \
      --iidfile "./.${image:?}-image-digest" \
      --load \
      --provenance "false" \
      "${@}" \
      "./ci/"

  digest="$("cat" "./.${image:?}-image-digest")"

  case "${digest:?}" in
    ("sha256:"*) ;;
    (*)
      "echo" \
        "Digest doesn't have expected format! Got: \"${digest:?}\"!" \
        >&2

      exit "1"
  esac

  images="$(
    "docker" \
      "image" \
      "ls" \
      --digests \
      --no-trunc
  )"

  id="$(
    "awk" \
      -v "digest=${digest:?}" \
      "\$3 == digest { print \$4; exit; }" \
      <<EOF
${images:?}
EOF
  )"

  unset "images"

  "docker" \
    "image" \
    "tag" \
    "${id:?}" \
    "localhost/local/${image:?}"
)
