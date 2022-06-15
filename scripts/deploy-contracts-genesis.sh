#!/bin/bash

# The functions require 'run_cmd' funcion to be available in the shell

add_wasm_messages() {
  local -r genesis_home_dir="$1"
  local -r wasm_code_path="$2"
  local -r admin_addr="$3"
  local -r treasury_init_tokens="$4"
  # TODO fill in other contracts

  run_cmd "$genesis_home_dir" add-wasm-genesis-message store "$wasm_code_path/treasury.wasm" --run-as "$admin_addr"
  run_cmd "$genesis_home_dir" add-wasm-genesis-message instantiate-contract 1 "{}" --label treasury \
                              --run-as "$admin_addr" --admin "$admin_addr" --amount "$treasury_init_tokens"
}

treasury_instance_addr() {
  # An instance address is computed as a function of the code ID and the globally incremented number of instantiations done so far.
  # A consequence of the above is that the instance address of smart contracts will not change when the code binary changes
  # unless the order is changed.

  # this the address of the first instatiation that is of the first deployed code, assuming that is treasury.
  # to update if the order is changed
  echo "nolus14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9s0k0puz"
}