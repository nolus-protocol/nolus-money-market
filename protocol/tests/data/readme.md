# Exports

## Test Prices

```sh
sh ../scripts/export-state-to-csv.sh --contract nolus1azehn8dlez772lqq5y0kv3q85c4ez8q0l8eq25q0kyd86q4rrmhsfqfnxf --output-file tests/data/dev-osmosis-oracle.txt --nolusd <path> --node https://dev-cl.nolus.network:26657/
```


## Test Oracle V1->V2 migration
```sh
sh ../scripts/export-state-to-csv.sh --contract nolus1azehn8dlez772lqq5y0kv3q85c4ez8q0l8eq25q0kyd86q4rrmhsfqfnxf --output-file tests/data/oracle_v1.txt --nolusd ../../nolus-core/target/release/nolusd --node https://dev-cl.nolus.network:26657/
```