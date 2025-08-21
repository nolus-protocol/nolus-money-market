#!/usr/bin/env sh

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
## Used utilities outside the POSIX standard:                                 ##
## cargo                                                                      ##
################################################################################

set -eu

(
  path="$("pwd")"

  workspace="$("basename" "${path:?}")"

  "echo" \
    "Workspace: ${workspace:?}" \
    >&2
)

case "${#}" in
  ("0") ;;
  (*)
    "echo" "This script takes no arguments!" >&2

    exit "1"
esac

if "test" -e ".ignore-lockfile"
then
  "echo" "Ignoring lockfile check."
else
  "cargo" "update" --locked
fi
