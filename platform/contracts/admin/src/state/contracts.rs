use std::collections::BTreeMap;

use sdk::{
    cosmwasm_ext::as_dyn::{storage, AsDyn},
    cosmwasm_std::{Addr, Order},
    cw_storage_plus::{Item, Map},
};

use crate::{
    contracts::{ContractsGroupedByProtocol, ContractsTemplate, PlatformTemplate, Protocol},
    error::Error,
    result::Result,
};

const PLATFORM: Item<'_, PlatformTemplate<Addr>> = Item::new("platform_contracts");
const PROTOCOL: Map<'_, String, Protocol> = Map::new("protocol_contracts");

pub(crate) fn store<S>(storage: &mut S, contracts: ContractsGroupedByProtocol) -> Result<()>
where
    S: storage::DynMut + ?Sized,
{
    PLATFORM
        .save(storage.as_dyn_mut(), &contracts.platform)
        .map_err(Into::into)
        .and_then(|()| {
            contracts.protocol.into_iter().try_for_each(
                |(protocol, ref contracts): (String, Protocol)| {
                    PROTOCOL
                        .save(storage.as_dyn_mut(), protocol, contracts)
                        .map_err(Into::into)
                },
            )
        })
}

pub(crate) fn add_protocol<S>(storage: &mut S, name: String, protocol: &Protocol) -> Result<()>
where
    S: storage::DynMut + ?Sized,
{
    if PROTOCOL.has(storage.as_dyn(), name.clone()) {
        Err(Error::ProtocolSetAlreadyExists(name))
    } else {
        PROTOCOL
            .save(storage.as_dyn_mut(), name, protocol)
            .map_err(Into::into)
    }
}

pub(crate) fn protocols<S>(storage: &S) -> Result<Vec<String>>
where
    S: storage::Dyn + ?Sized,
{
    PROTOCOL
        .keys(storage.as_dyn(), None, None, Order::Ascending)
        .collect::<std::result::Result<_, _>>()
        .map_err(Into::into)
}

pub(crate) fn load_platform<S>(storage: &S) -> Result<PlatformTemplate<Addr>>
where
    S: storage::Dyn + ?Sized,
{
    PLATFORM.load(storage.as_dyn()).map_err(Into::into)
}

pub(crate) fn load_protocol<S>(storage: &S, name: String) -> Result<Protocol>
where
    S: storage::Dyn + ?Sized,
{
    PROTOCOL.load(storage.as_dyn(), name).map_err(Into::into)
}

pub(crate) fn load_all<S>(storage: &S) -> Result<ContractsGroupedByProtocol>
where
    S: storage::Dyn + ?Sized,
{
    load_platform(storage).and_then(|platform: PlatformTemplate<Addr>| {
        PROTOCOL
            .range(storage.as_dyn(), None, None, Order::Ascending)
            .collect::<::std::result::Result<_, _>>()
            .map(|protocol: BTreeMap<String, Protocol>| ContractsTemplate { platform, protocol })
            .map_err(Into::into)
    })
}
