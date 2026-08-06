mod execute_admin;
mod execute_outbound;
mod instantiate;
mod migrate;
mod query;
mod scenarios;

use currencies::{
    LeaseGroup, Lpns as LpnGroup, PaymentGroup,
    testing::{PaymentC1, PaymentC2, PaymentC3},
};
use finance::duration::Duration;
use sdk::{
    cosmwasm_std::{
        Addr, Binary, ContractInfoResponse, ContractResult, Deps, DepsMut, MessageInfo, OwnedDeps,
        SystemError, SystemResult, WasmQuery,
        testing::{self, MockApi, MockQuerier, MockStorage},
    },
    testing as sdk_testing,
};
use versioning::VersionSegment;

use remote_lease::{Ics20ChannelId, msg::OpenLeaseParams};

use crate::{
    api::{ChannelResponse, ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg},
    contract::{instantiate, query},
    state::{Channel, ChannelState},
};

type OpenLeaseParamsT = OpenLeaseParams<LeaseGroup, LpnGroup, PaymentGroup>;
type ExecuteMsgT = ExecuteMsg<LeaseGroup, LpnGroup, PaymentGroup>;

const ADMIN: &str = "admin";
const NON_ADMIN: &str = "intruder";
const CREATOR: &str = "creator";
const CONNECTION_ID: &str = "connection-3";
const DEX_LABEL: &str = "osmosis";
const LEASE_CODE_ID: u64 = 17;
const WRONG_CODE_ID: u64 = LEASE_CODE_ID + 1;
const LEASE: &str = "lease";
const WRONG_CODE_CONTRACT: &str = "wrong-code-contract";
const NON_CONTRACT_CALLER: &str = "wallet-only";
const PACKET_TIMEOUT: Duration = Duration::from_secs(600);
const LOCAL_CHANNEL_ID: &str = "channel-0";
const COUNTERPARTY_CHANNEL_ID: &str = "channel-77";
const COUNTERPARTY_PORT_ID: &str = "nls-remote-lease.osmosis";
const ICS20_CHANNEL_REMOTE: &str = "channel-5";
// Literal on purpose: composing it from the wire crate here would let a
// grammar regression rewrite the expectation along with the code.
const PROPOSED_VERSION: &str = "nls-remote-lease.v1+transfer=channel-5";
const CONTRACT_STORAGE_VERSION: VersionSegment = 0;

fn deps() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    sdk_testing::mock_deps_with_contracts([])
}

// Wasm querier that resolves the two registered contract addresses to their
// distinct code ids; all other addresses return `NoSuchContract`. Shared
// between `execute_outbound` tests (which need the lease address authorised)
// and the first scenario in `scenarios` (which drives an outbound packet
// before consuming the ack).
fn deps_with_lease() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let lease = sdk_testing::user(LEASE);
    let wrong = sdk_testing::user(WRONG_CODE_CONTRACT);
    let mut deps = sdk_testing::mock_deps_with_contracts([]);
    deps.querier.update_wasm(move |query| match query {
        WasmQuery::ContractInfo { contract_addr } => {
            let addr = Addr::unchecked(contract_addr);
            if addr == lease {
                contract_info_response(LEASE_CODE_ID)
            } else if addr == wrong {
                contract_info_response(WRONG_CODE_ID)
            } else {
                SystemResult::Err(SystemError::NoSuchContract {
                    addr: contract_addr.clone(),
                })
            }
        }
        WasmQuery::CodeInfo { code_id } => SystemResult::Ok(ContractResult::Ok(
            sdk::cosmwasm_std::to_json_binary(&sdk::cosmwasm_std::CodeInfoResponse::new(
                *code_id,
                sdk_testing::user(""),
                sdk::cosmwasm_std::Checksum::generate(&[0x1f, 0x4e, 0x20, 0x9a]),
            ))
            .expect("serialization succeeds"),
        )),
        _ => unimplemented!(),
    });
    deps
}

fn contract_info_response(code_id: u64) -> SystemResult<ContractResult<Binary>> {
    SystemResult::Ok(ContractResult::Ok(
        sdk::cosmwasm_std::to_json_binary(&ContractInfoResponse::new(
            code_id,
            sdk_testing::user("creator"),
            None,
            false,
            None,
            None,
        ))
        .expect("serialization succeeds"),
    ))
}

fn instantiate_default(deps: DepsMut<'_>) {
    instantiate(
        deps,
        testing::mock_env(),
        sender(CREATOR),
        instantiate_msg(),
    )
    .unwrap();
}

fn established_channel() -> Channel {
    Channel::Established {
        local_channel_id: LOCAL_CHANNEL_ID.into(),
        counterparty_channel_id: COUNTERPARTY_CHANNEL_ID.into(),
        counterparty_port_id: COUNTERPARTY_PORT_ID.into(),
        ics20_channel_remote: ics20_channel_remote(ICS20_CHANNEL_REMOTE),
        state: ChannelState::Open,
    }
}

fn store_open_channel(deps: DepsMut<'_>) {
    established_channel().store(deps.storage).unwrap();
}

fn store_closing_channel(deps: DepsMut<'_>) {
    established_channel()
        .into_closing()
        .unwrap()
        .store(deps.storage)
        .unwrap();
}

fn store_proposal(deps: DepsMut<'_>) {
    Channel::proposed(ics20_channel_remote(ICS20_CHANNEL_REMOTE))
        .store(deps.storage)
        .unwrap();
}

fn store_init_accepted(deps: DepsMut<'_>) {
    Channel::proposed(ics20_channel_remote(ICS20_CHANNEL_REMOTE))
        .into_init_accepted(LOCAL_CHANNEL_ID.into())
        .unwrap()
        .store(deps.storage)
        .unwrap();
}

fn instantiate_msg() -> InstantiateMsg {
    InstantiateMsg {
        protocol_admin: sdk_testing::user(ADMIN).into_string(),
        connection_id: CONNECTION_ID.into(),
        dex_label: DEX_LABEL.into(),
        lease_code: LEASE_CODE_ID.into(),
    }
}

fn sender(who: &str) -> MessageInfo {
    MessageInfo {
        sender: sdk_testing::user(who),
        funds: vec![],
    }
}

fn query_config(deps: Deps<'_>) -> ConfigResponse {
    let raw = query(deps, testing::mock_env(), QueryMsg::Config()).unwrap();
    sdk::cosmwasm_std::from_json(raw).unwrap()
}

fn query_channel(deps: Deps<'_>) -> ChannelResponse {
    let raw = query(deps, testing::mock_env(), QueryMsg::Channel()).unwrap();
    sdk::cosmwasm_std::from_json(raw).unwrap()
}

fn ics20_channel_remote(ics20_channel: &str) -> Ics20ChannelId {
    ics20_channel.parse().expect("a canonical channel id")
}

fn sample_open_lease_params() -> OpenLeaseParamsT {
    OpenLeaseParams::new(
        7,
        currency::dto::<PaymentC1, PaymentGroup>(),
        currency::dto::<PaymentC2, LpnGroup>(),
        currency::dto::<PaymentC3, LeaseGroup>(),
    )
    .expect("sample uses three distinct currencies")
}
