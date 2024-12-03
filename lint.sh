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

RELEASE_VERSION=dev
readonly RELEASE_VERSION
export RELEASE_VERSION

case "${#:?}" in
  ("0")
    repo_root="$(cd "../" && "pwd")"
    ;;
  ("1")
    case "${1:?}" in
      ("--help")
        "echo" \
          "First argument of the script has to be the name of the workspace to \
run the linter in.
Without an argument, the current directory is assumed to the active workspace."

        exit "0"
        ;;
    esac

    repo_root="$("pwd")"

    cd "./${1:?}"

    shift
    ;;
  (*)
    "echo" \
      "Error!
\"${0:?}\" requires at most one argument, the name of the workspace!
Instead got ${#:?} arguments!" \
      >&2
    ;;
esac

readonly repo_root

. "${repo_root:?}/scripts/check/configuration.sh"

while read -r profile
do
  "cargo" \
    "each" \
    "run" \
    --external-command \
    --tag "ci" \
    -- \
    "sh" \
    "${repo_root:?}/scripts/check/lint.sh" \
    --profile "${profile:?}"
done <<EOF
${profiles:?}
EOF
