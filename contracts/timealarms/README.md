# Time Alarms


### Instantiate contract

* instantiate contract and verify
```
INIT='{}'
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

* subscribe for new time alarm
```
ADD_ALARM='{"addr" : <contract address>, "time": <timestamp>}'
nolusd tx wasm execute $CONTRACT "$ADD_ALARM" --amount 100unolus --from wallet $TXFLAG -y
```

* notify subscribed addresses
```
NOTIFY='{}'
nolusd tx wasm execute $CONTRACT "$NOTIFY" --amount 100unolus --from wallet $TXFLAG -y
```

