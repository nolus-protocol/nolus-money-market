#!/bin/sh -eu

################################################################################
## This script shall conform to the POSIX.1 standard, a.k.a. IEEE Std 1003.1. ##
## When utilities defined in the standard are to be invoked, they shall only  ##
## be invoked utilizing functions defined by the standard, excluding any and  ##
## all extensions to the standard functions, e.g. GNU extensions.             ##
##                                                                            ##
## Version of the POSIX.1 standard used: POSIX.1-2008                         ##
## https://pubs.opengroup.org/onlinepubs/9699919799.2008edition/              ##
##                                                                            ##
## Used version of the standard should not be moved forward unless necessary  ##
## in order to keep the script as portable as possible between different      ##
## environments.                                                              ##
##                                                                            ##
## Used version of the standard should be moved backwards if possible in      ##
## order to keep the script as portable as possible between different         ##
## environments.                                                              ##
################################################################################

set -eu

script="$("basename" "${0:?}")"
script_dir="${0%"${script:?}"}"
unset script
case "${script_dir?}" in
  ("/"*) ;;
  (*) script_dir="./${script_dir?}"
esac
readonly script_dir

mode="instance"
readonly mode
: "${mode:?}"

. "${script_dir:?}/setup.sh"

cargo_each() {
  "cargo" \
    -- \
    "each" \
    "run" \
    --tag "ci" \
    --tag "${dex_type:?}" \
    -- \
    "${@:?}"
}

case "${USE_NEXTEST-"0"}" in
  ("0")
    run_tests() {
      "cargo_each" \
        --quiet \
        "test" \
        --all-targets \
        --profile "${profile:?}" \
        --quiet
    }
    ;;
  ("1")
    run_tests() {
      "cargo_each" \
        "nextest" \
        "run" \
        --all-targets \
        --cargo-profile "${profile:?}" \
        --cargo-quiet \
        --final-status-level="fail" \
        --hide-progress-bar \
        --no-fail-fast \
        --no-tests="warn" \
        --retries "0" \
        --status-level "none" \
        --success-output "never"
    }
    ;;
  (*)
    "error" "Environment variable \"USE_NEXTEST\" set to a value other than 0 \
or 1!"
    ;;
esac

if run_tests
then
  "log" "Tests passed."
else
  "error" "Workspace \"${workspace:?}\" tests failed with profile \
\"${profile:?}\" and dex type \"${dex_type:?}\"!"
fi
