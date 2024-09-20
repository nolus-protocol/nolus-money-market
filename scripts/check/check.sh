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

. "${script_dir-""}/configuration.sh"

mode="runner"
readonly mode
: "${mode:?}"

. "${script_dir-""}/setup.sh"

check_with_workspace() {
  "log" "Running checks for workspace: ${1:?}"

  "run" \
    "sh" \
    -eu \
    "${script_dir-""}/workspace_checks.sh" \
    "${workspaces_dir-"/code"}/${1:?}/"

  "run" \
    "check_with_dex_type" \
    "${workspaces_dir-"/code"}/${1:?}/"
}

check_with_dex_type() {
  while read -r dex_type
  do
    "log" "Running checks for DEX type: ${dex_type:?}"

    "run" \
      "sh" \
      -eu \
      "${script_dir-""}/protocol_check_deps.sh" \
      "${1:?}" \
      "${dex_type:?}"

    "run" \
      "check_with_profile" \
      "${1:?}" \
      "${dex_type:?}"
  done \
    <<EOF
${dex_types:?}
EOF
}

check_with_profile() {
  while read -r profile
  do
    "log" "Running checks for profile: ${profile:?}"

    "run" \
      "sh" \
      -eu \
      "${script_dir-""}/instance_lint.sh" \
      "${1:?}" \
      "${2:?}" \
      "${profile:?}"

    "run" \
      "sh" \
      -eu \
      "${script_dir-""}/instance_tests.sh" \
      "${1:?}" \
      "${2:?}" \
      "${profile:?}"
  done \
    <<EOF
${profiles:?}
EOF
}

while read -r workspace
do
  "check_with_workspace" "${workspace:?}"
done \
  <<EOF
${workspaces:?}
EOF
