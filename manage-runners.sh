#!/usr/bin/env sh

set -eu

case "${1:?"Action not set"}" in
  ("add"|"remove") count="3" ;;
  ("start"|"stop") count="2" ;;
  (*)
    "echo" \
      "Unknown action!" \
      >&2

    exit "1"
esac

case "${#}" in
  ("${count:?}") ;;
  (*)
    "echo" \
      "Expected ${count} argument(s) to be passed! Got: ${#}!" \
      >&2

    exit "1"
esac

case "${1:?}" in
  ("add") "mkdir" "${2:?"Runner name not set!"}"
esac

cd "${2:?"Runner name not set!"}"

case "${1:?}" in
  ("add")
    "tar" \
      "xzf" \
      "../actions-runner.tar.gz"

    "./config.sh" \
      --url "https://github.com/KirilMihaylov/nolus-money-market" \
      --token "${3:?"Token not set!"}" <<EOF

ubuntu-vm-${2:?}


EOF
    ;;
  ("start")
    if test -e "./pid"
    then
      "echo" \
        "Runner already startd!" \
        >&2

      exit "1"
    fi

    RUNNER_MANUALLY_TRAP_SIG="1" \
      "./run.sh" &

    "echo" \
       "${!:?}" \
       >"./pid"
    ;;
  ("stop")
    if ! test -e "./pid"
    then
      "echo" \
        "Runner not started!" \
        >&2

      exit "1"
    fi

    pid="$("cat" "./pid")"

    while "kill" -2 "-${pid:?}"
    do
      "echo" "Sent stop signal."

      sleep "1"
    done

    "rm" "./pid"
    ;;
  ("remove")
    "./config.sh" \
      "remove" \
      --token "${3:?"Token not set!"}"

    cd ".."
esac
