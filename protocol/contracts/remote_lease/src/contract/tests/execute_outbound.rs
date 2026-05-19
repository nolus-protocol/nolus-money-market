use currencies::testing::{PaymentC1, PaymentC2, PaymentC3};
use finance::{coin::Coin, instant::Instant};
use remote_lease::{
    envelope::{LeaseAddrOnWire, PacketEnvelope},
    msg::{CloseLeaseParams, Operation, SwapParams, TransferOutParams},
    version::ProtocolVersion,
};
use sdk::{
    cosmwasm_ext::{CosmosMsg, SubMsg},
    cosmwasm_std::{IbcMsg, IbcTimeout, testing},
    testing as sdk_testing,
};

use crate::{api::ExecuteMsg, contract::execute, error::Error};

use super::{
    LEASE, LOCAL_CHANNEL_ID, NON_CONTRACT_CALLER, PACKET_TIMEOUT, WRONG_CODE_CONTRACT,
    deps_with_lease, instantiate_default, sample_open_lease_params, sender, store_closing_channel,
    store_open_channel,
};

#[test]
fn open_lease_emits_send_packet() {
    let mut deps = deps_with_lease();
    instantiate_default(deps.as_mut());
    store_open_channel(deps.as_mut());

    let params = sample_open_lease_params();
    let res = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(LEASE),
        ExecuteMsg::OpenLease {
            params: params.clone(),
            timeout: PACKET_TIMEOUT,
        },
    )
    .unwrap();
    assert_send_packet(&Operation::OpenLease(params), &res.messages);
}

#[test]
fn close_lease_emits_send_packet() {
    let mut deps = deps_with_lease();
    instantiate_default(deps.as_mut());
    store_open_channel(deps.as_mut());

    let params = CloseLeaseParams {};
    let res = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(LEASE),
        ExecuteMsg::CloseLease {
            params: params.clone(),
            timeout: PACKET_TIMEOUT,
        },
    )
    .unwrap();
    assert_send_packet(&Operation::CloseLease(params), &res.messages);
}

#[test]
fn swap_emits_send_packet() {
    let mut deps = deps_with_lease();
    instantiate_default(deps.as_mut());
    store_open_channel(deps.as_mut());

    let params = sample_swap_params();
    let res = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(LEASE),
        ExecuteMsg::Swap {
            params: params.clone(),
            timeout: PACKET_TIMEOUT,
        },
    )
    .unwrap();
    assert_send_packet(&Operation::Swap(params), &res.messages);
}

#[test]
fn transfer_out_emits_send_packet() {
    let mut deps = deps_with_lease();
    instantiate_default(deps.as_mut());
    store_open_channel(deps.as_mut());

    let params = sample_transfer_out_params();
    let res = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(LEASE),
        ExecuteMsg::TransferOut {
            params: params.clone(),
            timeout: PACKET_TIMEOUT,
        },
    )
    .unwrap();
    assert_send_packet(&Operation::TransferOut(params), &res.messages);
}

#[test]
fn outbound_when_no_channel_rejected() {
    let mut deps = deps_with_lease();
    instantiate_default(deps.as_mut());

    let err = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(LEASE),
        open_lease_execute(),
    )
    .unwrap_err();
    assert!(matches!(err, Error::ChannelNotOpen), "got {err:?}");
}

#[test]
fn outbound_when_channel_closing_rejected() {
    let mut deps = deps_with_lease();
    instantiate_default(deps.as_mut());
    store_closing_channel(deps.as_mut());

    let err = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(LEASE),
        open_lease_execute(),
    )
    .unwrap_err();
    assert!(matches!(err, Error::ChannelNotOperational), "got {err:?}");
}

#[test]
fn outbound_wrong_caller_code_rejected() {
    let mut deps = deps_with_lease();
    instantiate_default(deps.as_mut());
    store_open_channel(deps.as_mut());

    let err = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(WRONG_CODE_CONTRACT),
        open_lease_execute(),
    )
    .unwrap_err();
    assert!(matches!(err, Error::UnauthorisedCaller), "got {err:?}");
}

#[test]
fn outbound_non_contract_caller_rejected() {
    let mut deps = deps_with_lease();
    instantiate_default(deps.as_mut());
    store_open_channel(deps.as_mut());

    let err = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(NON_CONTRACT_CALLER),
        open_lease_execute(),
    )
    .unwrap_err();
    assert!(matches!(err, Error::UnauthorisedCaller), "got {err:?}");
}

fn open_lease_execute() -> ExecuteMsg {
    ExecuteMsg::OpenLease {
        params: sample_open_lease_params(),
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

fn assert_send_packet(expected_operation: &Operation, messages: &[SubMsg]) {
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
            assert_eq!(
                LeaseAddrOnWire::new(sdk_testing::user(LEASE)),
                envelope.lease,
            );
            assert_eq!(expected_operation, &envelope.operation);
            assert_eq!(ProtocolVersion, envelope.version);
        }
        other => panic!("expected CosmosMsg::Ibc(IbcMsg::SendPacket {{..}}), got {other:?}"),
    }
}

fn expected_timeout() -> IbcTimeout {
    use cw_time::{IntoInstant as _, IntoTimestamp as _};
    let now: Instant = testing::mock_env().block.time.into_instant();
    IbcTimeout::with_timestamp((now + PACKET_TIMEOUT).into_timestamp())
}
