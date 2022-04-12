# Oracle


### Instantiate contract

* instantiate contract and verify
```
INIT='{"base_asset":"NOL","price_feed_period":60,"feeders_percentage_needed":50}'
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


* register feeder address. Will use the treasury address just for the test
```
REGISTER='{"register_feeder":{"feeder_address":"nolus1qaf23chpkknx2znmz6p7k7n0u2uk5xtr5zdaf2"}}'
nolusd tx wasm execute $CONTRACT "$REGISTER" --amount 100unolus --from wallet $TXFLAG -y
```

* query name record
```
FEEDERS_QUERY='{"feeders" : {}}'
nolusd query wasm contract-state smart $CONTRACT "$FEEDERS_QUERY" --output json
```
