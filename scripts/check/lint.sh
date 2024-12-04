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

append_lint_flags() {
  "${@:?}" \
    -- \
    --allow "clippy::large_enum_variant" \
    --deny "clippy::all" \
    --deny "clippy::unwrap_used" \
    --deny "clippy::unwrap_in_result" \
    --deny "future-incompatible" \
    --deny "nonstandard-style" \
    --deny "refining-impl-trait" \
    --deny "rust-2018-idioms" \
    --deny "rust-2021-compatibility" \
    --deny "rust-2024-compatibility" \
    --allow "impl-trait-overcaptures" \
    --deny "unused" \
    --deny "warnings"
}

append_quiet_and_lints() {
  case "${RUN_CLIPPY_QUIET-"0"}" in
    ("0")
      "append_lint_flags" "${@:?}"
      ;;  
    ("1")
      "append_lint_flags" \
        "${@:?}" \
        --quiet
      ;;
    (*)
      "echo" \
        "Environment variable \"RUN_CLIPPY_QUIET\" is set to value other than \
zero or one!" \
        >&2
      ;;
  esac
}

"append_quiet_and_lints" \
  "cargo" \
  -- \
  "clippy" \
  --all-targets \
  "${@}"
