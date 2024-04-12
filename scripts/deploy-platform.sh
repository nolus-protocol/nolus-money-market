#!/bin/bash

# The functions require 'run_cmd' function to be available in the shell

add_wasm_messages() {
  local -r genesis_home_dir="$1"
  local -r wasm_code_path="$2"
  local -r treasury_init_tokens="$3"
  local -r dex_admin="$4"

  local -r TIMEALARMS_ADDRESS=$(timealarms_instance_addr)
  local -r TREASURY_ADDRESS=$(treasury_instance_addr)
  local -r ADMIN_CONTRACT_ADDRESS=$(admin_contract_instance_addr)

  local id=0

  local -r timealarms_init_msg='{}'
  _add_wasm_message "$genesis_home_dir" "$wasm_code_path" "timealarms" "$((++id))" \
    "$ADMIN_CONTRACT_ADDRESS" "" "--instantiate-anyof-addresses $ADMIN_CONTRACT_ADDRESS" "$timealarms_init_msg"

  local -r treasury_init_msg='{"cadence_hours":12,"protocols_registry":"'"$ADMIN_CONTRACT_ADDRESS"'","timealarms":"'"$TIMEALARMS_ADDRESS"'","tvl_to_apr":{"bars":[{"tvl":0,"apr":150},{"tvl":500,"apr":140},{"tvl":1000,"apr":130},{"tvl":2000,"apr":120},{"tvl":3000,"apr":110},{"tvl":4000,"apr":100},{"tvl":5000,"apr":90},{"tvl":7500,"apr":80},{"tvl":10000,"apr":70},{"tvl":15000,"apr":60},{"tvl":20000,"apr":50},{"tvl":25000,"apr":40},{"tvl":30000,"apr":30},{"tvl":40000,"apr":20}]}}'
  _add_wasm_message "$genesis_home_dir" "$wasm_code_path" "treasury" "$((++id))" \
    "$ADMIN_CONTRACT_ADDRESS" "$treasury_init_tokens"  "--instantiate-anyof-addresses $ADMIN_CONTRACT_ADDRESS" \
    "$treasury_init_msg"

  local -r admin_contract_init_msg='{"dex_admin":"'"${dex_admin}"'","contracts":{"platform":{"dispatcher":"'"${TIMEALARMS_ADDRESS}"'","timealarms":"'"${TIMEALARMS_ADDRESS}"'","treasury":"'"${TREASURY_ADDRESS}"'"},"protocol":{}}}'
  _add_wasm_message "$genesis_home_dir" "$wasm_code_path" "admin_contract" \
    "$((++id))" "$ADMIN_CONTRACT_ADDRESS" "" "--instantiate-anyof-addresses $ADMIN_CONTRACT_ADDRESS" \
    "$admin_contract_init_msg"
}

timealarms_instance_addr() {
  # An instance address is computed as a function of the code ID and the globally
  # incremented number of instantiations done so far.
  # A consequence of the above is that the instance address of smart contracts
  # will not change when the code binary changes unless the order is changed.

  # this the address of the first instatiation that is of the first deployed
  # code, assuming that is timealarms.
  # to update if the order is changed
  echo "nolus14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9s0k0puz"
}

treasury_instance_addr() {
  echo "nolus1nc5tatafv6eyq7llkr2gv50ff9e22mnf70qgjlv737ktmt4eswrqrr2r7y"
}

admin_contract_instance_addr() {
  echo "nolus17p9rzwnnfxcjp32un9ug7yhhzgtkhvl9jfksztgw5uh69wac2pgsmc5xhq"
}

_add_wasm_message() {
  local -r genesis_home_dir="$1"
  local -r wasm_code_path="$2"
  local -r contract_name="$3"
  local -r code_id="$4"
  local -r admin_addr="$5"
  local -r init_tokens="$6"
  local -r instantiate_policy="$7"

  if ! [ -f "$wasm_code_path/$contract_name.wasm" ]; then
    echo "The path '$wasm_code_path' does not contain the $contract_name " \
      "contracts' code."
    exit 1
  fi

  local amount_flag=""
  if ! [ "$init_tokens" = "" ]; then
      amount_flag="--amount $init_tokens"
  fi

  run_cmd "$genesis_home_dir" "add-wasm-genesis-message" "store" \
    "$wasm_code_path/$contract_name.wasm" "--run-as" "$admin_addr" \
    $instantiate_policy

  if [[ $# -eq 8 ]]; then
    local -r init_msg="$8"

    run_cmd "$genesis_home_dir" "add-wasm-genesis-message" "instantiate-contract" \
      "$code_id" "$init_msg" "--label" "$contract_name" "--run-as" "$admin_addr" \
      "--admin" "$admin_addr" $amount_flag
  fi
}
