#!/bin/sh

###################
## Prerequisites: #
## 'nolusd'       #
## 'jq'           #
###################

########################
## Inputs:             #
## '-b', '--nolusd'    #
## '-n', '--node'      #
## '-c', '--contract'  #
## '-o', 'output_file' #
########################

# Set default values
nolusd="nolusd"

nolusd_set="0"
node_set="0"
contract_set="0"
output_file_set="0"

check_too_few_args() {
  if [ "${1}" -lt 2 ]
  then
    echo "Too few parameters! Expected ${2}!"

    exit 1
  fi
}

ensure_not_set() {
  if [ "${1}" -ne 0 ]
  then
    echo "\"${2}\" is already set!"

    exit 1
  fi
}

while [ "${#}" -ne 0 ]
do
  case "${1}" in
  ("-b" | "--nolusd")
    check_too_few_args "${#}" "\"nolusd\" binary's path"
    ensure_not_set "${nolusd_set}" "--nolusd"

    nolusd_set="1"
    nolusd="${2}"

    shift 2
    ;;
  ("-n" | "--node")
    check_too_few_args "${#}" "node's URL"
    ensure_not_set "${node_set}" "--node"

    node_set="1"
    node="--node ${2}"

    shift 2
    ;;
  ("-c" | "--contract")
    check_too_few_args "${#}" "contract's address"
    ensure_not_set "${contract_set}" "--contract"

    contract_set="1"
    contract="${2}"

    shift 2
    ;;
  ("-o" | "--output-file")
    check_too_few_args "${#}" "contract's address"
    ensure_not_set "${output_file_set}" "--output_file"

    output_file_set="1"
    output_file="${2}"

    shift 2
    ;;
  (*)
    echo "Unknown parameter \"${1}\"!"

    exit 1
    ;;
  esac
done

ensure_is_set() {
  if [ "${1}" -eq 0 ]
  then
    echo "\"${2}\" is not set!"

    exit 1
  fi
}

ensure_is_set "${contract_set}" "--contract"
ensure_is_set "${output_file_set}" "--output-file"

unset -v "nolusd_set"
unset -v "node_set"
unset -v "contract_set"
unset -v "output_file_set"
unset -f "check_too_few_args"
unset -f "ensure_not_set"
unset -f "ensure_is_set"

exit_on_err() {
  # shellcheck disable=SC2181
  if [ "${?}" -ne 0 ]
  then
    code="${?}"

    echo "Error has occurred! Exiting!"

    exit "${code}"
  fi
}

command="${nolusd} q wasm cs all ${contract} --output json ${node}"

next_key=""

truncate --size 0 "${output_file}"

while true
do
  # shellcheck disable=SC2086
  response="$(${command} ${next_key})"

  exit_on_err

  output="$(
    echo "${response}" | jq ".models[] | .key + \",\" + .value" | tr -d "\""
  )"

  exit_on_err

  echo "${output}" >> "${output_file}"

  next_key="$(echo "${response}" | jq ".pagination.next_key" | tr -d "\"")"

  exit_on_err

  if [ "${next_key}" = "null" ]
  then
    break
  else
    next_key="--page-key ${next_key}"
  fi
done
