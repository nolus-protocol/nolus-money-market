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
script_dir="${script_dir?}/check/"
case "${script_dir?}" in
  ("/"*) ;;
  (*) script_dir="./${script_dir?}"
esac
readonly script_dir
export script_dir

workspaces_dir="${1:?"Script expects one argument specifying the path to the \
directory containing the workspaces!"}"
readonly workspaces_dir
export workspaces_dir
shift

case "${#}" in
  ("0") ;;
  (*) "error" "The script expects exactly one argument!"
esac

mode="runner"
readonly mode
: "${mode:?}"

. "${script_dir:?}/setup.sh"

"sh" \
  -eu \
  "${script_dir:?}/check.sh"
