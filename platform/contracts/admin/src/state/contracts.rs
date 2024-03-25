use std::collections::BTreeMap;

#[cfg(feature = "migrate")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "migrate")]
use sdk::cosmwasm_std::{Api, QuerierWrapper};
use sdk::{
    cosmwasm_std::{Addr, Order, Storage},
    cw_storage_plus::{Item, Map},
};

#[cfg(feature = "migrate")]
use crate::contracts::ProtocolTemplate;
use crate::{
    contracts::{ContractsGroupedByProtocol, ContractsTemplate, PlatformTemplate, Protocol},
    error::Error,
    result::Result,
};

const PLATFORM: Item<'_, PlatformTemplate<Addr>> = Item::new("platform_contracts");
const PROTOCOL: Map<'_, String, Protocol> = Map::new("protocol_contracts");

pub(crate) fn store(
    storage: &mut dyn Storage,
    contracts: ContractsGroupedByProtocol,
) -> Result<()> {
    PLATFORM
        .save(storage, &contracts.platform)
        .map_err(Into::into)
        .and_then(|()| {
            contracts.protocol.into_iter().try_for_each(
                |(protocol, ref contracts): (String, Protocol)| {
                    PROTOCOL
                        .save(storage, protocol, contracts)
                        .map_err(Into::into)
                },
            )
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

pub(crate) fn load_all(storage: &dyn Storage) -> Result<ContractsGroupedByProtocol> {
    load_platform(storage).and_then(|platform: PlatformTemplate<Addr>| {
        PROTOCOL
            .range(storage, None, None, Order::Ascending)
            .collect::<::std::result::Result<_, _>>()
            .map(|protocol: BTreeMap<String, Protocol>| ContractsTemplate { platform, protocol })
            .map_err(Into::into)
    })
}

#[cfg(feature = "migrate")]
pub(super) fn migrate_protocols(
    storage: &mut dyn Storage,
    api: &dyn Api,
    querier: QuerierWrapper<'_>,
    mut reserve_contracts: BTreeMap<String, String>,
) -> Result<()> {
    Map::<'_, String, OldProtocol>::new(
        std::str::from_utf8(PROTOCOL.namespace()).expect("Expected valid UTF-8 encoded key!"),
    )
    .range(storage, None, None, Order::Ascending)
    .collect::<sdk::cosmwasm_std::StdResult<Vec<_>>>()
    .map_err(Into::into)
    .and_then(|protocols| {
        protocols.into_iter().try_for_each(|(name, protocol)| {
            let Some(reserve) = reserve_contracts.remove(&name) else {
                return Err(Error::MissingProtocol(name));
            };

            api.addr_validate(&reserve)
                .map_err(Into::into)
                .and_then(|reserve| {
                    platform::contract::validate_addr(querier, &reserve)
                        .map(|()| reserve)
                        .map_err(Into::into)
                })
                .and_then(|reserve| {
                    PROTOCOL
                        .save(storage, name, &protocol.convert(reserve))
                        .map_err(Into::into)
                })
        })
    })
}

#[cfg(feature = "migrate")]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct OldProtocolTemplate<T> {
    pub leaser: T,
    pub lpp: T,
    pub oracle: T,
    pub profit: T,
}

#[cfg(feature = "migrate")]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct OldProtocol {
    pub network: String,
    pub contracts: OldProtocolTemplate<Addr>,
}

#[cfg(feature = "migrate")]
impl OldProtocol {
    fn convert(self, reserve: Addr) -> Protocol {
        Protocol {
            network: self.network,
            contracts: ProtocolTemplate {
                leaser: self.contracts.leaser,
                lpp: self.contracts.lpp,
                oracle: self.contracts.oracle,
                profit: self.contracts.profit,
                reserve,
            },
        }
    }
}
