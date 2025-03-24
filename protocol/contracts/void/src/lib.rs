use std::convert::Infallible;

use serde::{Deserialize, Serialize};

use platform::response;
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{
        Binary, Coin, Deps, DepsMut, Env, IbcAckCallbackMsg, IbcBasicResponse, IbcDstCallback,
        IbcSourceCallbackMsg, IbcSrcCallback, IbcTimeoutCallbackMsg, MessageInfo, StdError,
        StdResult, Timestamp, TransferMsgBuilder, entry_point,
    },
};

#[derive(Serialize, Deserialize)]
// deliberetly not #[serde(deny_unknown_fields)] to allow migration with any message
pub struct EmptyMsg {}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct InstantiateMsg {}

#[entry_point]
pub fn instantiate(
    _deps: DepsMut<'_>,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<CwResponse, StdError> {
    Ok(response::empty_response())
}

#[entry_point]
pub fn migrate(
    _deps: DepsMut<'_>,
    _env: Env,
    EmptyMsg {}: EmptyMsg,
) -> Result<CwResponse, platform::error::Error> {
    Ok(response::empty_response())
}

#[entry_point]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<CwResponse, StdError> {
    match msg {
        ExecuteMsg::SendIbcTransfer {
            recipient,
            amount,
            channel_id,
        } => send_ibc_transfer(deps, env, recipient, amount, channel_id),
    }
}

fn send_ibc_transfer(
    _deps: DepsMut<'_>,
    env: Env,
    recipient: String,
    amount: u128,
    channel_id: String,
) -> StdResult<CwResponse> {
    let msg = TransferMsgBuilder::new(
        channel_id,
        recipient.clone(),
        Coin::new(amount, "unls"),
        Timestamp::from_seconds(12345),
    )
    .with_src_callback(IbcSrcCallback {
        address: env.contract.address,
        gas_limit: None,
    })
    .with_dst_callback(IbcDstCallback {
        address: recipient.clone(),
        gas_limit: None,
    })
    .build();

    Ok(CwResponse::new()
        .add_message(msg)
        .add_attribute("action", "send_ibc_transfer"))
}

pub fn ibc_source_callback(
    _: DepsMut<'_>,
    _env: Env,
    msg: IbcSourceCallbackMsg,
) -> StdResult<IbcBasicResponse> {
    match msg {
        IbcSourceCallbackMsg::Acknowledgement(IbcAckCallbackMsg {
            acknowledgement: _,
            original_packet: _,
            relayer: _,
            ..
        }) => {
            // handle the acknowledgement
        }
        IbcSourceCallbackMsg::Timeout(IbcTimeoutCallbackMsg {
            packet: _,
            relayer: _,
            ..
        }) => {
            // handle the timeout
        }
    }

    Ok(IbcBasicResponse::new().add_attribute("action", "ibc_source_callback"))
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct IbcTransferMsg {
    pub recipient: String,
    pub amount: u128,
}

#[entry_point]
pub fn query(_deps: Deps<'_>, _env: Env, EmptyMsg {}: EmptyMsg) -> Result<Binary, Infallible> {
    unimplemented!("No query is availabve on a Void contract!");
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ExecuteMsg {
    SendIbcTransfer {
        recipient: String,
        amount: u128,
        channel_id: String,
    },
}
