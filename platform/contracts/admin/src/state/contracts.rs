use sdk::{
    cosmwasm_std::{Addr, Order, Storage},
    cw_storage_plus::{Item, Map},
};

use crate::{
    contracts::{Contracts, ContractsTemplate, PlatformContractAddressesWithoutAdmin, Protocol},
    error::Error,
    result::Result,
};

const PLATFORM: Item<PlatformContractAddressesWithoutAdmin> = Item::new("platform_contracts");

const PROTOCOL: Map<String, Protocol<Addr>> = Map::new("protocol_contracts");

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
        .collect::<Result<_, _>>()
        .map_err(Into::into)
}

pub(crate) fn load_platform(
    storage: &dyn Storage,
) -> Result<PlatformContractAddressesWithoutAdmin> {
    PLATFORM.load(storage).map_err(Into::into)
}

pub(crate) fn load_protocol(storage: &dyn Storage, name: String) -> Result<Protocol<Addr>> {
    PROTOCOL.load(storage, name).map_err(Into::into)
}

pub(crate) fn load_all(storage: &dyn Storage) -> Result<Contracts> {
    load_platform(storage).and_then(|platform| {
        PROTOCOL
            .range(storage, None, None, Order::Ascending)
            .collect::<Result<_, _>>()
            .map(|protocol| ContractsTemplate { platform, protocol })
            .map_err(Into::into)
    })
}
