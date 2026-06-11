use platform::contract::{Code, CodeId};
use sdk::{
    cosmwasm_std::{
        OwnedDeps, SystemError, SystemResult, Uint64, WasmQuery,
        testing::{self, MockApi, MockQuerier, MockStorage},
    },
    testing as sdk_testing,
};

use crate::{api::InstantiateMsg, contract::instantiate, error::Error};

use super::{
    CONNECTION_ID, CREATOR, DEX_LABEL, LEASE_CODE_ID, TRANSFER_CHANNEL, deps, instantiate_msg,
    query_channel, query_config, sender,
};

#[test]
fn proper_initialization() {
    let mut deps = deps();
    let res = instantiate(
        deps.as_mut(),
        testing::mock_env(),
        sender(CREATOR),
        instantiate_msg(),
    )
    .unwrap();
    assert_eq!(0, res.messages.len());

    let config = query_config(deps.as_ref());
    assert_eq!(CONNECTION_ID, config.connection_id);
    assert_eq!(DEX_LABEL, config.dex_label);
    assert_eq!(TRANSFER_CHANNEL, config.transfer_channel);
    assert_eq!(
        Uint64::from(CodeId::from(Code::unchecked(LEASE_CODE_ID))),
        config.lease_code_id,
    );

    let channel = query_channel(deps.as_ref());
    assert_eq!(None, channel.channel);
}

#[test]
fn instantiate_rejects_empty_connection_id() {
    let mut deps = deps();
    let msg = InstantiateMsg {
        connection_id: String::new(),
        ..instantiate_msg()
    };
    let err = instantiate(deps.as_mut(), testing::mock_env(), sender(CREATOR), msg).unwrap_err();
    assert!(
        matches!(err, Error::EmptyInstantiateField("connection_id")),
        "got {err:?}"
    );
}

#[test]
fn instantiate_rejects_empty_dex_label() {
    let mut deps = deps();
    let msg = InstantiateMsg {
        dex_label: String::new(),
        ..instantiate_msg()
    };
    let err = instantiate(deps.as_mut(), testing::mock_env(), sender(CREATOR), msg).unwrap_err();
    assert!(
        matches!(err, Error::EmptyInstantiateField("dex_label")),
        "got {err:?}"
    );
}

// Mirrors the Solana responder's canonical-channel-id rules: `channel-`
// prefix, decimal ordinal with no sign or leading zeros, within `u16` range.
#[test]
fn instantiate_rejects_non_canonical_transfer_channel() {
    const NON_CANONICAL: [&str; 8] = [
        "",
        "42",
        "channel-",
        "channel-abc",
        "channel-007",
        "channel-+5",
        "channel-70000",
        "transfer/channel-4",
    ];

    for transfer_channel in NON_CANONICAL {
        let mut deps = deps();
        let msg = InstantiateMsg {
            transfer_channel: transfer_channel.into(),
            ..instantiate_msg()
        };
        let err =
            instantiate(deps.as_mut(), testing::mock_env(), sender(CREATOR), msg).unwrap_err();
        assert!(
            matches!(err, Error::NonCanonicalTransferChannel(_)),
            "expected reject for {transfer_channel:?}, got {err:?}"
        );
    }
}

#[test]
fn instantiate_rejects_malformed_admin() {
    let mut deps = deps();
    let msg = InstantiateMsg {
        protocol_admin: "NOT_BECH32!".into(),
        ..instantiate_msg()
    };
    let err = instantiate(deps.as_mut(), testing::mock_env(), sender(CREATOR), msg).unwrap_err();
    assert!(matches!(err, Error::Std(_)), "got {err:?}");
}

#[test]
fn instantiate_rejects_unknown_lease_code() {
    let mut deps = deps_with_failing_code_info();
    let err = instantiate(
        deps.as_mut(),
        testing::mock_env(),
        sender(CREATOR),
        instantiate_msg(),
    )
    .unwrap_err();
    assert!(matches!(err, Error::Platform(_)), "got {err:?}");
}

// Querier that returns `NoSuchCode` for every `CodeInfo` query. The contract's
// `Code::try_new` resolves to a `CodeInfo` query under the hood; replacing the
// default closure (which would unconditionally return a happy `CodeInfoResponse`)
// is the only way to exercise that error arm.
fn deps_with_failing_code_info() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = sdk_testing::mock_deps_with_contracts([]);
    deps.querier.update_wasm(|query| match query {
        WasmQuery::CodeInfo { code_id } => {
            SystemResult::Err(SystemError::NoSuchCode { code_id: *code_id })
        }
        _ => unimplemented!("unexpected wasm query in this test"),
    });
    deps
}
