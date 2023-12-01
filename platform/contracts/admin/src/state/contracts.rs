use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_std::{Addr, Order, Storage},
    cw_storage_plus::{Item, Map},
};

use crate::{
    contracts::{
        ContractsGroupedByProtocol, ContractsTemplate, Platform, Protocol, ProtocolWithNetworkName,
    },
    error::Error,
    result::Result,
};

const PLATFORM: Item<'_, Platform<Addr>> = Item::new("platform_contracts");
const PROTOCOL: Map<'_, String, ProtocolWithNetworkName> = Map::new("protocol_contracts");

pub(crate) fn store(
    storage: &mut dyn Storage,
    contracts: ContractsGroupedByProtocol,
) -> Result<()> {
    PLATFORM
        .save(storage, &contracts.platform)
        .map_err(Into::into)
        .and_then(|()| {
            contracts.protocol.into_iter().try_for_each(
                |(protocol, ref contracts): (String, ProtocolWithNetworkName)| {
                    PROTOCOL
                        .save(storage, protocol, contracts)
                        .map_err(Into::into)
                },
            )
        })
}

pub(crate) fn add_protocol_set(
    storage: &mut dyn Storage,
    name: String,
    protocol: &ProtocolWithNetworkName,
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

pub(crate) fn load_platform(storage: &dyn Storage) -> Result<Platform<Addr>> {
    PLATFORM.load(storage).map_err(Into::into)
}

pub(crate) fn load_protocol(
    storage: &dyn Storage,
    name: String,
) -> Result<ProtocolWithNetworkName> {
    PROTOCOL.load(storage, name).map_err(Into::into)
}

pub(crate) fn load_all(storage: &dyn Storage) -> Result<ContractsGroupedByProtocol> {
    load_platform(storage).and_then(|platform: Platform<Addr>| {
        PROTOCOL
            .range(storage, None, None, Order::Ascending)
            .collect::<::std::result::Result<_, _>>()
            .map(
                |protocol: BTreeMap<String, ProtocolWithNetworkName>| ContractsTemplate {
                    platform,
                    protocol,
                },
            )
            .map_err(Into::into)
    })
}

pub(crate) fn migrate(
    storage: &mut dyn Storage,
    protocol_name: String,
    network_name: String,
) -> Result<()> {
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
                            protocol_name,
                            &ProtocolWithNetworkName {
                                network: network_name,
                                protocol: Protocol {
                                    leaser,
                                    lpp,
                                    oracle,
                                    profit,
                                },
                            },
                        )
                    })
            },
        )
        .map_err(Into::into)
}
