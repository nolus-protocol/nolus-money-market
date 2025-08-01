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
## cargo [with:]                                                              ##
##   * rustc                                                                  ##
##   * clippy                                                                 ##
################################################################################

set -eu

# Deny/Forbid Lint Groups
# \
# Deny/Forbid Individual lints
# \
# Deny/Forbid Clippy Lints
# \
# Deny Forbid "warnings" Lint Group
# \
# Allowed Lints
"cargo" \
  "clippy" \
  --all-targets \
  "${@:?}" \
  --profile "${PROFILE:?}" \
  -- \
  --forbid "deprecated-safe" \
  --deny "future-incompatible" \
  --deny "keyword-idents" \
  --deny "nonstandard-style" \
  --deny "refining-impl-trait" \
  --deny "rust-2018-idioms" \
  --deny "unused" \
  \
  --forbid "unfulfilled_lint_expectations" \
  \
  --deny "clippy::all" \
  --deny "clippy::unwrap_used" \
  --deny "clippy::unwrap_in_result" \
  \
  --deny "warnings" \
  \
  --allow "clippy::large_enum_variant"
