#!/bin/bash

# The functions require 'run_cmd' function to be available in the shell

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
source "$SCRIPT_DIR"/common/cmd.sh
source "$SCRIPT_DIR"/common/deployment.sh
source "$SCRIPT_DIR"/common/tx.sh

deploy_contracts() {
  local -r nolus_net="$1"
  local -r chain_id="$2"
  local -r nolus_home_dir="$3"
  local -r wasm_code_path="$4"
  local -r dex_admin_wallet_key="$5"
  local -r store_code_privileged_wallet_key="$6"

  local -r TIMEALARMS_ADDRESS=$(timealarms_instance_addr)
  local -r TREASURY_ADDRESS=$(treasury_instance_addr)
  local -r ADMIN_CONTRACT_ADDRESS=$(admin_contract_instance_addr)

  FLAGS="--broadcast-mode sync --fees 200000unls --gas auto --gas-adjustment 1.2 --node $nolus_net --chain-id $chain_id --home $nolus_home_dir"

  local id=0

  local -r timealarms_init_msg='{}'
  _deploy_contract "$nolus_net" "$nolus_home_dir" "$wasm_code_path" "timealarms" "$((++id))" \
    "$timealarms_init_msg"  "$ADMIN_CONTRACT_ADDRESS" "$store_code_privileged_wallet_key" "$dex_admin_wallet_key"

  local -r treasury_init_msg='{"cadence_hours":12,"protocols_registry":"'"$ADMIN_CONTRACT_ADDRESS"'","timealarms":"'"$TIMEALARMS_ADDRESS"'","tvl_to_apr":{"bars":[{"tvl":0,"apr":150},{"tvl":500,"apr":140},{"tvl":1000,"apr":130},{"tvl":2000,"apr":120},{"tvl":3000,"apr":110},{"tvl":4000,"apr":100},{"tvl":5000,"apr":90},{"tvl":7500,"apr":80},{"tvl":10000,"apr":70},{"tvl":15000,"apr":60},{"tvl":20000,"apr":50},{"tvl":25000,"apr":40},{"tvl":30000,"apr":30},{"tvl":40000,"apr":20}]}}'
  _deploy_contract "$nolus_net" "$nolus_home_dir" "$wasm_code_path" "treasury" "$((++id))" \
    "$treasury_init_msg"  "$ADMIN_CONTRACT_ADDRESS" "$store_code_privileged_wallet_key" "$dex_admin_wallet_key"

  local -r dex_admin_address=$(run_cmd "$nolus_home_dir" keys show -a "$dex_admin_wallet_key")
  local -r admin_contract_init_msg='{"dex_admin":"'"${dex_admin_address}"'","contracts":{"platform":{"timealarms":"'"${TIMEALARMS_ADDRESS}"'","treasury":"'"${TREASURY_ADDRESS}"'"},"protocol":{}}}'
  _deploy_contract "$nolus_net" "$nolus_home_dir" "$wasm_code_path" "admin_contract" "$((++id))" \
    "$admin_contract_init_msg" "$ADMIN_CONTRACT_ADDRESS" "$store_code_privileged_wallet_key" "$dex_admin_wallet_key"
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

_deploy_contract() {
  local -r nolus_net="$1"
  local -r nolus_home_dir="$2"
  local -r wasm_code_path="$3"
  local -r contract_name="$4"
  local -r code_id="$5"
  local -r init_msg="$6"
  local -r admin_addr="$7"
  local -r store_code_privileged_wallet_key="$8"
  local -r instantiate_privileged_wallet_key="$9"

  if ! [ -f "$wasm_code_path/$contract_name.wasm" ]; then
    echo "The path '$wasm_code_path' does not contain the $contract_name " \
      "contracts' code."
    exit 1
  fi

  store_code "$nolus_net" "$nolus_home_dir" "$store_code_privileged_wallet_key" "$wasm_code_path/$contract_name.wasm" "--instantiate-anyof-addresses $instantiate_privileged_wallet_key"

  local -r instantiate_result=$(run_cmd "$nolus_home_dir" tx wasm instantiate "$code_id" "$init_msg" --admin "$admin_addr" --label "$contract_name" --from "$instantiate_privileged_wallet_key" $FLAGS --yes --output json)
  local -r instantiate_tx_hash=$(get_tx_hash "$instantiate_result")
  wait_tx_included_in_block "$nolus_home_dir" "$nolus_net" "$instantiate_tx_hash"
}
