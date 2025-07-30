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
  (*)
    working_dir="$(pwd)"
    readonly working_dir
    script_dir="${working_dir:?}/${script_dir?}"
esac
readonly script_dir

mode="instance"
readonly mode
: "${mode:?}"

. "${script_dir:?}/setup.sh"

if "cargo" \
  -- \
  "each" \
  "run" \
  --external-command \
  --tag "ci" \
  --tag "${dex_type:?}" \
  -- \
  "sh" \
  -eu \
  "${script_dir:?}/lint.sh" \
  --profile "${profile:?}"
then
  "log" "Linter checks passed."
else
  "error" "Workspace \"${workspace:?}\"'s linter checks failed with dex type \
\"${dex_type:?}\" and profile \"${profile:?}\"!"
fi
