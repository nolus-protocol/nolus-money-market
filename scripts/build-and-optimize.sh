#!/bin/sh

RUSTFLAGS="-C link-arg=-s ${RUSTFLAGS}"

rm -rf "/target/"

mkdir "/target/"

for contract in "/code/contracts/"*
do
    cd "${contract}"

    cargo build --release --lib --locked --target wasm32-unknown-unknown --target-dir "/target/"
done

if test $? -eq 0
then
    rm -rf "/artifacts/"*

    for wasm in "/target/wasm32-unknown-unknown/release/"*".wasm"
    do
        wasm_name=$(basename "${wasm}" ".wasm")

        echo "[INFO] Processing \"${wasm_name}\" through \"wasm-opt\"..."

        wasm-opt -Os --signext-lowering -o "/artifacts/${wasm_name}.wasm" "${wasm}"
    done

    sha256sum -- "/artifacts/"*".wasm" | tee "/artifacts/checksums.txt"
else
    echo "[ERROR] Cargo exitted with non-zero status code!"
fi

rm -rf "/target/"
