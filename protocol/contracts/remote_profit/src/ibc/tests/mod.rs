mod handshake;
mod packets;

use sdk::{
    cosmwasm_std::{
        IbcChannel, IbcEndpoint, IbcOrder, MessageInfo, OwnedDeps,
        testing::{self, MockApi, MockQuerier, MockStorage},
    },
    testing as sdk_testing,
};

use crate::{api::InstantiateMsg, contract::instantiate};

const ADMIN: &str = "admin";
const CREATOR: &str = "creator";
const CONNECTION_ID: &str = "connection-3";
const WRONG_CONNECTION_ID: &str = "connection-9";
const DEX_LABEL: &str = "osmosis";
const LOCAL_PORT_ID: &str = "wasm.controller";
const LOCAL_CHANNEL_ID: &str = "channel-0";
const COUNTERPARTY_CHANNEL_ID: &str = "channel-77";
const COUNTERPARTY_PORT_ID: &str = "nls-remote-profit.osmosis";
const WRONG_COUNTERPARTY_PORT_ID: &str = "nls-remote-profit.evil";
const TRANSFER_CHANNEL: &str = "channel-42";
const VERSION: &str = "nls-remote-profit.v1+transfer=channel-42";
const WRONG_VERSION: &str = "nls-remote-profit.v2+transfer=channel-42";
const BARE_VERSION: &str = "nls-remote-profit.v1";
const WRONG_TRANSFER_VERSION: &str = "nls-remote-profit.v1+transfer=channel-7";
const PROFIT_CODE_ID: u64 = 17;
const PROFIT_CONTRACT: &str = "profit-contract";

fn deps_with_config() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = sdk_testing::mock_deps_with_contracts([]);
    instantiate(
        deps.as_mut(),
        testing::mock_env(),
        MessageInfo {
            sender: sdk_testing::user(CREATOR),
            funds: vec![],
        },
        InstantiateMsg {
            protocol_admin: sdk_testing::user(ADMIN).into_string(),
            connection_id: CONNECTION_ID.into(),
            dex_label: DEX_LABEL.into(),
            transfer_channel: TRANSFER_CHANNEL.into(),
            profit_code: PROFIT_CODE_ID.into(),
            profit_contract: sdk_testing::user(PROFIT_CONTRACT).into_string(),
        },
    )
    .unwrap();
    deps
}

fn channel(
    order: IbcOrder,
    version: &str,
    connection_id: &str,
    counterparty_port_id: &str,
) -> IbcChannel {
    IbcChannel::new(
        IbcEndpoint {
            port_id: LOCAL_PORT_ID.into(),
            channel_id: LOCAL_CHANNEL_ID.into(),
        },
        IbcEndpoint {
            port_id: counterparty_port_id.into(),
            channel_id: COUNTERPARTY_CHANNEL_ID.into(),
        },
        order,
        version,
        connection_id,
    )
}
