use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_std::{Addr, Order, Storage},
    cw_storage_plus::{Item, Map},
};

use crate::{
    contracts::{Contracts, ContractsTemplate, PlatformContracts, Protocol},
    error::Error,
    result::Result,
};

const PLATFORM: Item<'_, PlatformContracts<Addr>> = Item::new("platform_contracts");
const PROTOCOL: Map<'_, String, Protocol<Addr>> = Map::new("protocol_contracts");

pub(crate) fn store(storage: &mut dyn Storage, contracts: Contracts) -> Result<()> {
    PLATFORM
        .save(storage, &contracts.platform)
        .map_err(Into::into)
        .and_then(|()| {
            contracts
                .protocol
                .into_iter()
                .try_for_each(|(name, protocol)| {
                    PROTOCOL.save(storage, name, &protocol).map_err(Into::into)
                })
        })
}

pub(crate) fn add_protocol(
    storage: &mut dyn Storage,
    name: String,
    protocol: &Protocol<Addr>,
) -> Result<()> {
    if PROTOCOL.has(storage, name.clone()) {
        Err(Error::ProtocolSetAlreadyExists(name))
    } else {
        PROTOCOL.save(storage, name, protocol).map_err(Into::into)
    }
}

pub(crate) fn remove_protocol(storage: &mut dyn Storage, name: String) {
    PROTOCOL.remove(storage, name)
}

pub(crate) fn protocols(storage: &dyn Storage) -> Result<Vec<String>> {
    PROTOCOL
        .keys(storage, None, None, Order::Ascending)
        .collect::<std::result::Result<_, _>>()
        .map_err(Into::into)
}

pub(crate) fn load_platform(storage: &dyn Storage) -> Result<PlatformContracts<Addr>> {
    PLATFORM.load(storage).map_err(Into::into)
}

pub(crate) fn load_protocol(storage: &dyn Storage, name: String) -> Result<Protocol<Addr>> {
    PROTOCOL.load(storage, name).map_err(Into::into)
}

pub(crate) fn load_all(storage: &dyn Storage) -> Result<Contracts> {
    load_platform(storage).and_then(|platform| {
        PROTOCOL
            .range(storage, None, None, Order::Ascending)
            .collect::<::std::result::Result<_, _>>()
            .map(|protocol| ContractsTemplate { platform, protocol })
            .map_err(Into::into)
    })
}

pub(super) fn migrate_platform(storage: &mut dyn Storage) -> Result<()> {
    #[derive(Serialize, Deserialize)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct Platform {
        pub dispatcher: Addr,
        pub timealarms: Addr,
        pub treasury: Addr,
    }

    Item::<'_, Platform>::new(
        std::str::from_utf8(PLATFORM.as_slice()).expect("Expected valid UTF-8 encoded key!"),
    )
    .load(storage)
    .map_err(Into::into)
    .and_then(
        |Platform {
             dispatcher: _,
             timealarms,
             treasury,
         }| {
            PLATFORM
                .save(
                    storage,
                    &PlatformContracts {
                        timealarms,
                        treasury,
                    },
                )
                .map_err(Into::into)
        },
    )
}
