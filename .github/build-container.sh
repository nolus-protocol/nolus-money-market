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

  SOURCE_DATE_EPOCH="0" \
    "docker" \
      "buildx" \
      "build" \
      "${@:?}" \
      --build-arg "SOURCE_DATE_EPOCH=0" \
      --file "./ci/images/${image:?}.Containerfile" \
      --iidfile "./.${friendly_name:?}-image-digest" \
      "./ci/"
)

build_rust() (
  rust_version="${1:?"No Rust version selected!"}"

  "build" \
    "rust-${rust_version:?}" \
    "rust-${rust_version:?}" \
    -- \
    --tag "localhost/local/rust-${rust_version:?}"

  "cat" "./.rust-${rust_version:?}-image-digest"
)
