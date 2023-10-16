use std::collections::BTreeMap;

use sdk::{
    cosmwasm_std::{Addr, Order, Storage},
    cw_storage_plus::{Item, Map},
};
use serde::{Deserialize, Serialize};

use crate::{
    common::type_defs::{Contracts, DexBoundContracts, DexIndependentContracts},
    result::Result as ContractResult,
    ContractError,
};

const DEX_INDEPENDENT: Item<'_, DexIndependentContracts> = Item::new("dex_independent_contracts");
const DEX_BOUND: Map<'_, String, DexBoundContracts> = Map::new("dex_bound_contracts");

pub(crate) fn store(storage: &mut dyn Storage, contracts: Contracts) -> ContractResult<()> {
    DEX_INDEPENDENT
        .save(storage, &contracts.dex_independent)
        .and_then(|()| {
            contracts.dex_bound.into_iter().try_for_each(
                |(dex, dex_bound): (String, DexBoundContracts)| {
                    DEX_BOUND.save(storage, dex, &dex_bound)
                },
            )
        })
        .map_err(Into::into)
}

pub(crate) fn add_dex_bound_set(
    storage: &mut dyn Storage,
    dex: String,
    contracts: &DexBoundContracts,
) -> ContractResult<()> {
    if DEX_BOUND.has(storage, dex.clone()) {
        Err(ContractError::DexSetAlreadyExists(dex))
    } else {
        DEX_BOUND.save(storage, dex, contracts).map_err(Into::into)
    }
}

pub(crate) fn load(storage: &dyn Storage) -> ContractResult<Contracts> {
    DEX_INDEPENDENT
        .load(storage)
        .and_then(|dex_independent: DexIndependentContracts| {
            DEX_BOUND
                .range(storage, None, None, Order::Ascending)
                .collect::<Result<_, _>>()
                .map(|dex_bound: BTreeMap<String, DexBoundContracts>| Contracts {
                    dex_independent,
                    dex_bound,
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
                    dex_independent: DexIndependentContracts {
                        dispatcher,
                        timealarms,
                        treasury,
                    },
                    dex_bound: BTreeMap::from([(
                        dex,
                        DexBoundContracts {
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
