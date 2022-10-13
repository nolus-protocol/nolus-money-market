#!/bin/sh

cat "./currencies.json" | cargo "run" "--package" "nolus-config" "--" "gen-curr" "--output-dir" "./packages/currency/src"
