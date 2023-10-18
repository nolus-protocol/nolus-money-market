use std::collections::BTreeMap;

use sdk::{
    cosmwasm_std::{Addr, Order, Storage},
    cw_storage_plus::{Item, Map},
};
use serde::{Deserialize, Serialize};

use crate::{
    common::type_defs::{Contracts, PlatformContracts, ProtocolContracts},
    result::Result as ContractResult,
    ContractError,
};

const PLATFORM: Item<'_, PlatformContracts> = Item::new("platform_contracts");
const PROTOCOL: Map<'_, String, ProtocolContracts> = Map::new("protocol_contracts");

pub(crate) fn store(storage: &mut dyn Storage, contracts: Contracts) -> ContractResult<()> {
    PLATFORM
        .save(storage, &contracts.platform)
        .and_then(|()| {
            contracts.protocol.into_iter().try_for_each(
                |(dex, dex_bound): (String, ProtocolContracts)| {
                    PROTOCOL.save(storage, dex, &dex_bound)
                },
            )
        })
        .map_err(Into::into)
}

pub(crate) fn add_dex_bound_set(
    storage: &mut dyn Storage,
    dex: String,
    contracts: &ProtocolContracts,
) -> ContractResult<()> {
    if PROTOCOL.has(storage, dex.clone()) {
        Err(ContractError::DexSetAlreadyExists(dex))
    } else {
        PROTOCOL.save(storage, dex, contracts).map_err(Into::into)
    }
}

pub(crate) fn load(storage: &dyn Storage) -> ContractResult<Contracts> {
    PLATFORM
        .load(storage)
        .and_then(|platform: PlatformContracts| {
            PROTOCOL
                .range(storage, None, None, Order::Ascending)
                .collect::<Result<_, _>>()
                .map(|protocol: BTreeMap<String, ProtocolContracts>| Contracts {
                    platform,
                    protocol,
                })
        })
        .map_err(Into::into)
}

pub(crate) fn migrate(storage: &mut dyn Storage, dex: String) -> ContractResult<()> {
    #[derive(Serialize, Deserialize)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    struct OldContracts {
        pub dispatcher: Addr,
        pub leaser: Addr,
        pub lpp: Addr,
        pub oracle: Addr,
        pub profit: Addr,
        pub timealarms: Addr,
        pub treasury: Addr,
    }

    const CONTRACTS: Item<'_, OldContracts> = Item::new("contracts");

    CONTRACTS.load(storage).map_err(Into::into).and_then(
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

            store(
                storage,
                Contracts {
                    platform: PlatformContracts {
                        dispatcher,
                        timealarms,
                        treasury,
                    },
                    protocol: BTreeMap::from([(
                        dex,
                        ProtocolContracts {
                            leaser,
                            lpp,
                            oracle,
                            profit,
                        },
                    )]),
                },
            )
        },
    )
}
