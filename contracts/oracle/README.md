# Oracle


### Instantiate contract

* instantiate contract and verify
```
INIT='{"base_asset":"B","price_feed_period":60,"feeders_percentage_needed":50,"supported_denom_pairs":[["A","B"],["A","C"],["C","D"]]}'
nolusd tx wasm instantiate $CODE_ID "$INIT" --from wallet --label "awesome oracle" $TXFLAG -y
```

* check the contract state (and account balance)
```
nolusd query wasm list-contract-by-code $CODE_ID --output json
CONTRACT=$(nolusd query wasm list-contract-by-code $CODE_ID --output json | jq -r '.contracts[-1]')
echo $CONTRACT
```

### Execute and Query

* we should see this contract with 0unolus
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
nolusd tx wasm execute $CONTRACT "$CONFIG_UPDATE" --amount 100unolus --from wallet $TXFLAG -y
```

* register feeder address. Will use the treasury address just for the test
```
WALLET_ADDR=$(nolusd keys show -a wallet)
REGISTER='{"register_feeder":{"feeder_address":"'$WALLET_ADDR'"}}'
nolusd tx wasm execute $CONTRACT "$REGISTER" --amount 100unolus --from wallet $TXFLAG -y
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

* update supported denom pairs
```
DENOM_PAIRS_UPDATE='{"supported_denom_pairs" : {"pairs": [["B","D"],["X","Y"],["C","D"]]}}'
nolusd tx wasm execute $CONTRACT "$DENOM_PAIRS_UPDATE" --amount 100unolus --from wallet $TXFLAG -y
```

* Push new price feed
```
FEED_PRICES='{"feed_prices":{"prices":[{"base":"A","values":[{"denom": "B", "amount": "1.2"},{"denom": "C", "amount": "2.1"}]},{"base":"C","values":[{"denom": "D", "amount": "3.2"}]}]}}'
nolusd tx wasm execute $CONTRACT "$FEED_PRICES" --amount 100unolus --from wallet $TXFLAG -y
```

* Query price feeds. Returns price against the base asset (taken from contract configuration)
```
PRICE='{"price_for":{"denoms": ["A"]}}'
nolusd query wasm contract-state smart  $CONTRACT "$PRICE" --output json
```
