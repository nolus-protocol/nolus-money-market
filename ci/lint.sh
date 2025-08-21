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
## [in-tree] cargo-each                                                       ##
## [in-tree] lint.workspace.sh                                                ##
## cargo [with:]                                                              ##
##   * Rust compiler                                                          ##
##   * clippy [inherited from 'lint.workspace.sh']                            ##
################################################################################

set -eu

case "${#}" in
  ("1") ;;
  (*)
    "echo" \
      "This script takes only one argument, the workspace name!" \
      >&2

    exit "1"
esac

cd "./${1:?}"
shift

"cargo" \
  -- \
  "each" \
  "run" \
  --external-command \
  -- \
  "lint.workspace.sh"
