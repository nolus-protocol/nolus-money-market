use platform::contract::Code;
use sdk::{
    cosmwasm_std::{
        Deps, MessageInfo, OwnedDeps, SystemError, SystemResult, WasmQuery,
        testing::{self, MockApi, MockQuerier, MockStorage},
    },
    testing as sdk_testing,
};

use crate::{
    api::{ExecuteMsg, InstantiateMsg},
    contract::{execute, instantiate},
    error::Error,
    state::Config,
};

const ADMIN: &str = "admin";
const NON_ADMIN: &str = "intruder";
const CREATOR: &str = "creator";
const LEASE_CODE_ID: u64 = 17;

#[test]
fn new_lease_code_admin_succeeds() {
    let mut deps = deps();
    instantiate_default(&mut deps);

    let new_code = Code::unchecked(LEASE_CODE_ID + 5);
    let res = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(ADMIN),
        ExecuteMsg::NewLeaseCode(new_code),
    )
    .unwrap();
    assert_eq!(0, res.messages.len());
    assert_stored_lease_code(new_code, deps.as_ref());
}

#[test]
fn new_lease_code_non_admin_rejected() {
    let mut deps = deps();
    instantiate_default(&mut deps);

    let err = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(NON_ADMIN),
        ExecuteMsg::NewLeaseCode(Code::unchecked(LEASE_CODE_ID + 1)),
    )
    .unwrap_err();
    assert!(matches!(err, Error::Unauthorized(_)), "got {err:?}");
    assert_stored_lease_code(Code::unchecked(LEASE_CODE_ID), deps.as_ref());
}

#[test]
fn new_lease_code_rejects_unknown_code() {
    let mut deps = deps();
    instantiate_default(&mut deps);

    // The rotated code id now resolves to no on-chain code, so the existence
    // check must reject it and leave the stored code untouched.
    deps.querier.update_wasm(|query| match query {
        WasmQuery::CodeInfo { code_id } => {
            SystemResult::Err(SystemError::NoSuchCode { code_id: *code_id })
        }
        _ => unimplemented!("unexpected wasm query in this test"),
    });

    let err = execute(
        deps.as_mut(),
        testing::mock_env(),
        sender(ADMIN),
        ExecuteMsg::NewLeaseCode(Code::unchecked(LEASE_CODE_ID + 9)),
    )
    .unwrap_err();
    assert!(matches!(err, Error::Platform(_)), "got {err:?}");
    assert_stored_lease_code(Code::unchecked(LEASE_CODE_ID), deps.as_ref());
}

fn deps() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    sdk_testing::mock_deps_with_contracts([])
}

fn instantiate_default(deps: &mut OwnedDeps<MockStorage, MockApi, MockQuerier>) {
    instantiate(
        deps.as_mut(),
        testing::mock_env(),
        sender(CREATOR),
        instantiate_msg(),
    )
    .unwrap();
}

fn instantiate_msg() -> InstantiateMsg {
    InstantiateMsg {
        protocol_admin: sdk_testing::user(ADMIN).into_string(),
        lease_code: LEASE_CODE_ID.into(),
    }
}

fn sender(who: &str) -> MessageInfo {
    MessageInfo {
        sender: sdk_testing::user(who),
        funds: vec![],
    }
}

fn assert_stored_lease_code(expected: Code, deps: Deps<'_>) {
    assert_eq!(expected, Config::load(deps.storage).unwrap().lease_code());
}
