use currencies::testing::{PaymentC1, PaymentC2, PaymentC3};
use finance::{coin::Coin, instant::Instant};
use remote_profit::{
    envelope::PacketEnvelope,
    msg::{CloseProfitParams, Operation, SwapParams, TransferOutParams},
    version::ProtocolVersion,
};
use sdk::{
    cosmwasm_ext::{CosmosMsg, SubMsg},
    cosmwasm_std::{IbcMsg, IbcTimeout, testing},
};

use crate::{api::ExecuteMsg, contract::execute, error::Error};

use super::{
    LOCAL_CHANNEL_ID, NON_CONTRACT_CALLER, PACKET_TIMEOUT, PROFIT, WRONG_CODE_CONTRACT,
    deps_with_profit, instantiate_default, sample_open_profit_params, sender,
    store_closing_channel, store_open_channel,
};

#[test]
fn open_profit_emits_send_packet() {
    let mut deps = deps_with_profit();
    instantiate_default(deps.as_mut());
    store_open_channel(deps.as_mut());

    let params = sample_open_profit_params();
    let res = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(PROFIT),
        ExecuteMsg::OpenProfit {
            params: params.clone(),
            timeout: PACKET_TIMEOUT,
        },
    )
    .unwrap();
    assert_send_packet(&Operation::OpenProfit(params), 0, &res.messages);
}

#[test]
fn close_profit_emits_send_packet() {
    let mut deps = deps_with_profit();
    instantiate_default(deps.as_mut());
    store_open_channel(deps.as_mut());

    let params = CloseProfitParams {};
    let res = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(PROFIT),
        ExecuteMsg::CloseProfit {
            params: params.clone(),
            timeout: PACKET_TIMEOUT,
        },
    )
    .unwrap();
    assert_send_packet(&Operation::CloseProfit(params), 0, &res.messages);
}

#[test]
fn swap_emits_send_packet() {
    const SWAP_NONCE: u64 = 7;

    let mut deps = deps_with_profit();
    instantiate_default(deps.as_mut());
    store_open_channel(deps.as_mut());

    let params = sample_swap_params();
    let res = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(PROFIT),
        ExecuteMsg::Swap {
            params: params.clone(),
            timeout: PACKET_TIMEOUT,
            nonce: SWAP_NONCE,
        },
    )
    .unwrap();
    assert_send_packet(&Operation::Swap(params), SWAP_NONCE, &res.messages);
}

#[test]
fn transfer_out_emits_send_packet() {
    const TRANSFER_OUT_NONCE: u64 = 5;

    let mut deps = deps_with_profit();
    instantiate_default(deps.as_mut());
    store_open_channel(deps.as_mut());

    let params = sample_transfer_out_params();
    let res = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(PROFIT),
        ExecuteMsg::TransferOut {
            params: params.clone(),
            timeout: PACKET_TIMEOUT,
            nonce: TRANSFER_OUT_NONCE,
        },
    )
    .unwrap();
    assert_send_packet(
        &Operation::TransferOut(params),
        TRANSFER_OUT_NONCE,
        &res.messages,
    );
}

#[test]
fn outbound_when_no_channel_rejected() {
    let mut deps = deps_with_profit();
    instantiate_default(deps.as_mut());

    let err = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(PROFIT),
        open_profit_execute(),
    )
    .unwrap_err();
    assert!(matches!(err, Error::ChannelNotOpen), "got {err:?}");
}

#[test]
fn outbound_when_channel_closing_rejected() {
    let mut deps = deps_with_profit();
    instantiate_default(deps.as_mut());
    store_closing_channel(deps.as_mut());

    let err = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(PROFIT),
        open_profit_execute(),
    )
    .unwrap_err();
    assert!(matches!(err, Error::ChannelNotOperational), "got {err:?}");
}

#[test]
fn outbound_wrong_caller_code_rejected() {
    let mut deps = deps_with_profit();
    instantiate_default(deps.as_mut());
    store_open_channel(deps.as_mut());

    let err = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(WRONG_CODE_CONTRACT),
        open_profit_execute(),
    )
    .unwrap_err();
    assert!(matches!(err, Error::UnauthorisedCaller), "got {err:?}");
}

#[test]
fn outbound_non_contract_caller_rejected() {
    let mut deps = deps_with_profit();
    instantiate_default(deps.as_mut());
    store_open_channel(deps.as_mut());

    let err = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(NON_CONTRACT_CALLER),
        open_profit_execute(),
    )
    .unwrap_err();
    assert!(matches!(err, Error::UnauthorisedCaller), "got {err:?}");
}

fn open_profit_execute() -> ExecuteMsg {
    ExecuteMsg::OpenProfit {
        params: sample_open_profit_params(),
        timeout: PACKET_TIMEOUT,
    }
}

fn sample_swap_params() -> SwapParams {
    SwapParams::new(
        Coin::<PaymentC1>::new(1_000).into(),
        Coin::<PaymentC2>::new(42).into(),
    )
    .expect("sample uses two distinct non-zero amounts")
}

fn sample_transfer_out_params() -> TransferOutParams {
    TransferOutParams::new(Coin::<PaymentC3>::new(1_000).into())
        .expect("sample uses a non-zero amount")
}

fn assert_send_packet(expected_operation: &Operation, expected_nonce: u64, messages: &[SubMsg]) {
    assert_eq!(1, messages.len(), "expected exactly one outbound message");
    match &messages[0].msg {
        CosmosMsg::Ibc(IbcMsg::SendPacket {
            channel_id,
            data,
            timeout,
        }) => {
            assert_eq!(LOCAL_CHANNEL_ID, channel_id);
            assert_eq!(&expected_timeout(), timeout);
            let envelope: PacketEnvelope = sdk::cosmwasm_std::from_json(data).unwrap();
            // The singleton profit envelope carries NO addressee identity (unlike
            // the multi-instance lease) — only the operation, version pin, and
            // per-emission nonce.
            assert_eq!(expected_operation, &envelope.operation);
            assert_eq!(ProtocolVersion, envelope.version);
            assert_eq!(expected_nonce, envelope.nonce);
        }
        other => panic!("expected CosmosMsg::Ibc(IbcMsg::SendPacket {{..}}), got {other:?}"),
    }
}

fn expected_timeout() -> IbcTimeout {
    use cw_time::{IntoInstant as _, IntoTimestamp as _};
    let now: Instant = testing::mock_env().block.time.into_instant();
    IbcTimeout::with_timestamp((now + PACKET_TIMEOUT).into_timestamp())
}
