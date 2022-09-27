# Oracle


### Instantiate contract

* Send some money to wasm_admin account
```
nolusd tx bank send $(nolusd keys show -a reserve) $(nolusd keys show -a wasm_admin) 1000000unls --chain-id nolus-local --keyring-backend test --fees 500unls
```
* Set some environment variables
```
export CHAIN_ID="nolus-local"
export TXFLAG="--chain-id ${CHAIN_ID} --gas auto --gas-adjustment 1.3 --fees 15000unls"
```

* instantiate contract and verify
```
INIT='{"base_asset":"B","price_feed_period":60,"feeders_percentage_needed":50,"currency_paths":[["A","B"],["C","D","B"],["E","F","G","B"]]}'
nolusd tx wasm instantiate $CODE_ID "$INIT" --from wasm_admin --label "awesome oracle" $TXFLAG -y
```
currency_paths - is a list of resolution paths. For example: the way to resolve the price of C, first we need to get the price of C against D, then the price of D against B, where B is the base asset for the contract. Every resolution path should end with the base asset


* check the contract state (and account balance)
```
nolusd query wasm list-contract-by-code $CODE_ID --output json
CONTRACT=$(nolusd query wasm list-contract-by-code $CODE_ID --output json | jq -r '.contracts[-1]')
echo $CONTRACT
```

### Execute and Query

* we should see this contract with 0unls
```
nolusd query wasm contract $CONTRACT
nolusd query bank balances $CONTRACT
```

* you can dump entire contract state
```
nolusd query wasm contract-state all $CONTRACT
```

note that we prefix the key "config" with two bytes indicating its length
```
echo -n config | xxd -ps
gives 636f6e666967
thus we have a key 0006636f6e666967
```

* you can also query one key directly
```
nolusd query wasm contract-state raw $CONTRACT 0006636f6e666967 --hex
```

* Note that keys are hex encoded, and val is base64 encoded.
To view the returned data (assuming it is ascii), try something like:
(Note that in many cases the binary data returned is non in ascii format, thus the encoding)
```
nolusd query wasm contract-state all $CONTRACT --output "json" | jq -r '.models[0].key' | xxd -r -ps
nolusd query wasm contract-state all $CONTRACT --output "json" | jq -r '.models[0].value' | base64 -d
```

* show oracle configuration
```
CONFIG_QUERY='{"config" : {}}'
nolusd query wasm contract-state smart $CONTRACT "$CONFIG_QUERY" --output json
```

* update oracle configuration
```
CONFIG_UPDATE='{"config" : {"price_feed_period":120,"feeders_percentage_needed":20}}'
nolusd tx wasm execute $CONTRACT "$CONFIG_UPDATE" --amount 100unls --from wasm_admin $TXFLAG -y
```

* register feeder address. Only the contract owner should be able to register new feeder address. All contracts are deployed from wasm_admin
```
WALLET_ADDR=$(nolusd keys show -a wallet)
REGISTER='{"register_feeder":{"feeder_address":"'$WALLET_ADDR'"}}'
nolusd tx wasm execute $CONTRACT "$REGISTER" --amount 100unls --from wasm_admin $TXFLAG -y
```

* remove registered feeders. Only contract owner
```
REMOVE='{"remove_feeder":{"feeder_address":"'$WALLET_ADDR'"}}'
nolusd tx wasm execute $CONTRACT "$REMOVE" --amount 100unls --from wasm_admin $TXFLAG -y
```

* query registered feeders
```
FEEDERS_QUERY='{"feeders" : {}}'
nolusd query wasm contract-state smart $CONTRACT "$FEEDERS_QUERY" --output json
```

* query supported denom pairs
```
DENOM_PAIRS_QUERY='{"supported_denom_pairs" : {}}'
nolusd query wasm contract-state smart $CONTRACT "$DENOM_PAIRS_QUERY" --output json
```

* update supported currency paths
```
CURRENCY_PATHS_UPDATE='{"currency_paths":[["A","B"],["C","D","B"],["E","F","G","B"]]}'
nolusd tx wasm execute $CONTRACT "$CURRENCY_PATHS_UPDATE" --amount 100unls --from wallet $TXFLAG -y
```

* Push new price feed
```
FEED_PRICES='{"feed_prices":{"prices":[{"amount":{"amount": "10", "symbol": "unls"}, "amount_quote":{"amount": "100", "symbol": "uusdc"}}]}}'
nolusd tx wasm execute $CONTRACT "$FEED_PRICES" --amount 100unls --from wallet $TXFLAG -y --fees 600unls
```

* Query price feeds. Returns price against the base asset (taken from contract configuration)
```
PRICE='{"price":{"currency": "OSMO"}}'
nolusd query wasm contract-state smart  $CONTRACT "$PRICE" --output json
```
* Query price feeds. Returns price for multiple denoms against the base asset (taken from contract configuration)
```
PRICES='{"prices":{"currencies": ["OSMO","ATOM"]}}'
nolusd query wasm contract-state smart  $CONTRACT "$PRICES" --output json
```
