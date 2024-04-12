use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_std::{Addr, Order, Storage},
    cw_storage_plus::{Item, Map},
};

use crate::{
    contracts::{
        Contracts, ContractsTemplate, Dex, Network, PlatformTemplate, Protocol, ProtocolTemplate,
    },
    error::Error,
    result::Result,
};

const PLATFORM: Item<'_, PlatformTemplate<Addr>> = Item::new("platform_contracts");
const PROTOCOL: Map<'_, String, Protocol> = Map::new("protocol_contracts");

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
    protocol: &Protocol,
) -> Result<()> {
    if PROTOCOL.has(storage, name.clone()) {
        Err(Error::ProtocolSetAlreadyExists(name))
    } else {
        PROTOCOL.save(storage, name, protocol).map_err(Into::into)
    }
}

pub(crate) fn protocols(storage: &dyn Storage) -> Result<Vec<String>> {
    PROTOCOL
        .keys(storage, None, None, Order::Ascending)
        .collect::<std::result::Result<_, _>>()
        .map_err(Into::into)
}

pub(crate) fn load_platform(storage: &dyn Storage) -> Result<PlatformTemplate<Addr>> {
    PLATFORM.load(storage).map_err(Into::into)
}

pub(crate) fn load_protocol(storage: &dyn Storage, name: String) -> Result<Protocol> {
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

pub(super) fn migrate_protocols(
    storage: &mut dyn Storage,
    mut dexes: BTreeMap<String, Dex>,
) -> Result<()> {
    Map::<'_, String, OldProtocol>::new(
        std::str::from_utf8(PROTOCOL.namespace()).expect("Expected valid UTF-8 encoded key!"),
    )
    .range(storage, None, None, Order::Ascending)
    .collect::<sdk::cosmwasm_std::StdResult<Vec<_>>>()
    .map_err(Into::into)
    .and_then(|protocols| {
        protocols.into_iter().try_for_each(|(name, protocol)| {
            let Some(dex) = dexes.remove(&name) else {
                return Err(Error::MissingProtocol(name));
            };

            PROTOCOL
                .save(storage, name, &protocol.migrate(dex))
                .map_err(Into::into)
        })
    })
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct OldProtocol {
    pub network: Network,
    pub contracts: ProtocolTemplate<Addr>,
}

impl OldProtocol {
    fn migrate(self, dex: Dex) -> Protocol {
        Protocol {
            network: self.network,
            dex,
            contracts: self.contracts,
        }
    }
}
