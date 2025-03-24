use std::convert::Infallible;

use serde::{Deserialize, Serialize};

use platform::response;
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo,
        // sIbcMsg, IbcOrder, IbcPacket, IbcReceiveResponse,
        StdError, StdResult, TransferMsgBuilder, IbcSourceCallbackMsg,
        IbcBasicResponse, IbcAckCallbackMsg, IbcTimeoutCallbackMsg,
        IbcDestinationCallbackMsg, IbcSrcCallback, ensure_eq, Coin,
        Timestamp, IbcDstCallback, BankMsg, StdAck,
        coins, from_json, entry_point},
};
use ibc::apps::transfer::types::packet::PacketData as TransferPacketData;

// use timealarms::msg::ExecuteAlarmMsg;

#[derive(Serialize, Deserialize)]
// deliberetly not #[serde(deny_unknown_fields)] to allow migration with any message
pub struct EmptyMsg {}

#[entry_point]
pub fn instantiate(
    _deps: DepsMut<'_>,
    _env: Env,
    _info: MessageInfo,
    EmptyMsg {}: EmptyMsg,
) -> Result<CwResponse, Infallible> {
    unimplemented!("Instantiation of a Void contract is not allowed!");
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
        } => send_ibc_transfer(deps, env, recipient, amount, channel_id)
    }
}

fn send_ibc_transfer(
    _deps: DepsMut,
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
     
    Ok(CwResponse::new().add_message(msg).add_attribute("action", "send_ibc_transfer"))
}

pub fn ibc_source_callback(
    deps: DepsMut,
    _env: Env,
    msg: IbcSourceCallbackMsg,
) -> StdResult<IbcBasicResponse> {
    match msg {
        IbcSourceCallbackMsg::Acknowledgement(IbcAckCallbackMsg {
            acknowledgement,
            original_packet,
            relayer,
            ..
        }) => {
            // handle the acknowledgement
        }
        IbcSourceCallbackMsg::Timeout(IbcTimeoutCallbackMsg {
            packet, relayer, ..
        }) => {
            // handle the timeout
        }
    }
 
    Ok(IbcBasicResponse::new().add_attribute("action", "ibc_source_callback"))
}
 
// pub fn ibc_destination_callback(
//     deps: DepsMut,
//     env: Env,
//     msg: IbcDestinationCallbackMsg,
// ) -> StdResult<IbcBasicResponse> {
//     ensure_eq!(
//         msg.packet.dest.port_id,
//         "transfer", // transfer module uses this port by default
//         StdError::generic_err("only want to handle transfer packets")
//     );
//     ensure_eq!(
//         msg.ack.data,
//         StdAck::success(b"\x01").to_binary(), // this is how a successful transfer ack looks
//         StdError::generic_err("only want to handle successful transfers")
//     );
//     // At this point we know that this is a callback for a successful transfer,
//     // but not to whom it is going, how much and what denom.
 
//     // Parse the packet data to get that information:
//     let packet_data: TransferPacketData = from_json(&msg.packet.data)?;
 
//     // The receiver should be a valid address on this chain.
//     // Remember, we are on the destination chain.
//     let receiver = deps.api.addr_validate(packet_data.receiver.as_ref())?;
//     ensure_eq!(
//         receiver,
//         env.contract.address,
//         StdError::generic_err("only want to handle transfers to this contract")
//     );
 
//     // We only care about this chain's native token in this example.
//     // The `packet_data.token.denom` is formatted as `{port id}/{channel id}/{denom}`,
//     // where the port id and channel id are the source chain's identifiers.
//     // Assuming we are running this on Neutron and only want to handle NTRN tokens,
//     // the denom should look like this:
//     let ntrn_denom = format!(
//         "{}/{}/untrn",
//         msg.packet.src.port_id, msg.packet.src.channel_id
//     );
//     ensure_eq!(
//         packet_data.token.denom.to_string(),
//         ntrn_denom,
//         StdError::generic_err("only want to handle NTRN tokens")
//     );
 
//     // Now, we can do something with our tokens.
//     // For example, we could send them to some address:
//     let msg = BankMsg::Send {
//         to_address: "neutron155exr8rqjrknusllpzxdfvezxr8ddpqehj9g9d".to_string(),
//         // this panics if the amount is too large
//         amount: coins(packet_data.token.amount.as_ref().as_u128(), "untrn"),
//     };
 
//     Ok(IbcBasicResponse::new()
//         .add_message(msg)
//         .add_attribute("action", "ibc_destination_callback"))
// }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct IbcTransferMsg {
    pub recipient: String,
    pub amount: u128,
}

#[entry_point]
pub fn query(_deps: Deps<'_>, _env: Env, EmptyMsg {}: EmptyMsg) -> Result<Binary, Infallible> {
    unimplemented!("No query is availabve on a Void contract!");
}

// Handle IBC channel open
// #[entry_point]
// pub fn ibc_channel_open(
//     _deps: DepsMut,
//     _env: Env,
//     msg: cosmwasm_std::IbcChannelOpenMsg,
// ) -> StdResult<()> {
//     if msg.channel.order != IbcOrder::Unordered {
//         return Err(StdError::generic_err("Only unordered channels are supported"));


//     }
//     Ok(())
// }

// // Handle IBC channel connect
// #[entry_point]
// pub fn ibc_channel_connect(
//     _deps: DepsMut,
//     _env: Env,
//     msg: cosmwasm_std::IbcChannelConnectMsg,
// ) -> StdResult<Response> {
//     Ok(Response::new()
//         .add_attribute("action", "ibc_channel_connect")
//         .add_attribute("channel_id", msg.channel.id))
// }

// // Handle IBC packet receive
// #[entry_point]
// pub fn ibc_packet_receive(
//     _deps: DepsMut,
//     _env: Env,
//     packet: IbcPacket,
// ) -> StdResult<IbcReceiveResponse> {
//     let msg: IbcTransferMsg = cosmwasm_std::from_binary(&packet.data)?;
//     Ok(IbcReceiveResponse::new()
//         .add_attribute("action", "ibc_packet_receive")
//         .add_attribute("recipient", msg.recipient)
//         .add_attribute("amount", msg.amount.to_string()))
// }

// // Handle IBC packet acknowledgment
// #[entry_point]
// pub fn ibc_packet_ack(
//     _deps: DepsMut,
//     _env: Env,
//     packet: IbcPacket,
//     ack: Binary,
// ) -> StdResult<Response> {
//     Ok(Response::new()
//         .add_attribute("action", "ibc_packet_ack")
//         .add_attribute("packet", String::from_utf8_lossy(&packet.data)))
// }

// // Handle IBC packet timeout
// #[entry_point]
// pub fn ibc_packet_timeout(
//     _deps: DepsMut,
//     _env: Env,
//     packet: IbcPacket,
// ) -> StdResult<Response> {
//     Ok(Response::new()
//         .add_attribute("action", "ibc_packet_timeout")
//         .add_attribute("packet", String::from_utf8_lossy(&packet.data)))
// }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ExecuteMsg {
    SendIbcTransfer {
        recipient: String,
        amount: u128,
        channel_id: String,
    },
}