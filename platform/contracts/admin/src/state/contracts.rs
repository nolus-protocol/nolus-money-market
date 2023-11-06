use std::collections::BTreeMap;

use platform::never::safe_unwrap;
use sdk::{
    cosmwasm_std::{Order, StdError as CwError, Storage},
    cw_storage_plus::{Item, Map},
};
use serde::{Deserialize, Serialize};

use crate::{
    common::{
        type_defs::ContractsGroupedByDex, CheckedAddr, ContractsTemplate, Platform, Protocol,
        StoredAddr, Transform as _,
    },
    result::Result as ContractResult,
    ContractError,
};

const PLATFORM: Item<'_, Platform<StoredAddr>> = Item::new("platform_contracts");
const PROTOCOL: Map<'_, String, Protocol<StoredAddr>> = Map::new("protocol_contracts");

pub(crate) fn store(
    storage: &mut dyn Storage,
    contracts: ContractsGroupedByDex,
) -> ContractResult<()> {
    PLATFORM
        .save(storage, &safe_unwrap(contracts.platform.transform(&())))
        .map_err(Into::into)
        .and_then(|()| {
            contracts.protocol.into_iter().try_for_each(
                |(dex, protocol): (String, Protocol<CheckedAddr>)| {
                    PROTOCOL
                        .save(storage, dex, &safe_unwrap(protocol.transform(&())))
                        .map_err(Into::into)
                },
            )
        })
}

pub(crate) fn add_dex_bound_set(
    storage: &mut dyn Storage,
    dex: String,
    contracts: Protocol<CheckedAddr>,
) -> ContractResult<()> {
    if PROTOCOL.has(storage, dex.clone()) {
        Err(ContractError::DexSetAlreadyExists(dex))
    } else {
        PROTOCOL
            .save(storage, dex, &safe_unwrap(contracts.transform(&())))
            .map_err(Into::into)
    }
}

pub(crate) fn load(storage: &dyn Storage) -> ContractResult<ContractsGroupedByDex> {
    PLATFORM
        .load(storage)
        .and_then(|platform: Platform<StoredAddr>| {
            PROTOCOL
                .range(storage, None, None, Order::Ascending)
                .map(|result: Result<(String, Protocol<StoredAddr>), CwError>| {
                    result.map(|(dex, protocol): (String, Protocol<StoredAddr>)| {
                        (dex, safe_unwrap(protocol.transform(&())))
                    })
                })
                .collect::<Result<_, _>>()
                .map(
                    |protocol: BTreeMap<String, Protocol<CheckedAddr>>| ContractsTemplate {
                        platform: safe_unwrap(platform.transform(&())),
                        protocol,
                    },
                )
        })
        .map_err(Into::into)
}

pub(crate) fn migrate(storage: &mut dyn Storage, dex: String) -> ContractResult<()> {
    #[derive(Serialize, Deserialize)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    struct OldContracts {
        pub dispatcher: StoredAddr,
        pub leaser: StoredAddr,
        pub lpp: StoredAddr,
        pub oracle: StoredAddr,
        pub profit: StoredAddr,
        pub timealarms: StoredAddr,
        pub treasury: StoredAddr,
    }

    const CONTRACTS: Item<'_, OldContracts> = Item::new("contracts");

    CONTRACTS
        .load(storage)
        .and_then(
            |OldContracts {
                 dispatcher,
                 leaser,
                 lpp,
                 oracle,
                 profit,
                 timealarms,
                 treasury,
             }: OldContracts| {
                CONTRACTS.remove(storage);

                PLATFORM
                    .save(
                        storage,
                        &Platform {
                            dispatcher,
                            timealarms,
                            treasury,
                        },
                    )
                    .and_then(|()| {
                        PROTOCOL.save(
                            storage,
                            dex,
                            &Protocol {
                                leaser,
                                lpp,
                                oracle,
                                profit,
                            },
                        )
                    })
            },
        )
        .map_err(Into::into)
}
