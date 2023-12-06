#!/bin/bash

RUSTFLAGS="-C link-arg=-s ${RUSTFLAGS}"

cd '/code/'

cargo update --locked

if ! test $? -eq 0
then
    echo '[ERROR] `Cargo.lock` file is either missing or is out-of-date!'

    exit 1
fi

rm -rf "/target/"*

rm -rf '/artifacts/'*

rm -rf '/temp-artifacts/'*

for contract in $(echo '/code/contracts/'* | sed 's/ /\n/g' | sort)
do
    cd "${contract}" || exit 1

    contract_pkgid="$(cargo pkgid)"
    contract_pkgid="${contract_pkgid##*/}"
    contract_pkgid_filter_at="${contract_pkgid%%@*}"
    if test "${contract_pkgid}" = "${contract_pkgid_filter_at}"
    then
        contract_pkgid="${contract_pkgid%%#*}"
    else
        contract_pkgid="${contract_pkgid_filter_at##*#}"
    fi
    unset 'contract_pkgid_filter_at'

    pkg_features="${contract_pkgid}_features"

    cargo build --release --lib --locked --target 'wasm32-unknown-unknown' --target-dir '/target/' --features "$(cargo features intersection "${features},${!pkg_features}")"

    if ! test $? -eq 0
    then
        echo "[ERROR] Cargo exitted with non-zero status code while being ran against \"${contract_pkgid}\"!"

        exit 1
    fi

    wasm-opt -Os --signext-lowering -o "/temp-artifacts/${contract_pkgid}.wasm" "/target/wasm32-unknown-unknown/release/${contract_pkgid}.wasm"

    if ! test $? -eq 0
    then
        echo "[ERROR] \"wasm-opt\" exitted with non-zero status code while being ran against \"${contract_pkgid}\"!"

        exit 1
    fi
done

mv -t '/artifacts/' '/temp-artifacts/'*'.wasm'

echo

echo 'Checksums:'

sha256sum -- '/artifacts/'*'.wasm' | tee '/artifacts/checksums.txt'

rm -rf '/target/'*
