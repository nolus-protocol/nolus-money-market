use admin_contract::{
    msg::{
        Dex, Network, ProtocolContractAddresses, ProtocolQueryResponse, ProtocolsQueryResponse,
        QueryMsg,
    },
    result::Result as ContractResult,
};
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{Addr, Binary, Deps, DepsMut, Empty, Env, MessageInfo, to_json_binary},
    testing,
};

use super::{ADMIN, CwContractWrapper, MockQueryMsg, dummy_query, test_case::app::App};

pub(crate) type QueryFn =
    fn(deps: Deps<'_, Empty>, env: Env, msg: QueryMsg) -> ContractResult<Binary>;

pub(crate) enum Registry {
    NoProtocol,
    SingleProtocol,
    TwoProtocols,
}

impl From<Registry> for QueryFn {
    fn from(value: Registry) -> Self {
        match value {
            Registry::NoProtocol => no_protocols_query,
            Registry::SingleProtocol => single_protocol_query,
            Registry::TwoProtocols => two_protocols_query,
        }
    }
}

pub(crate) struct Instantiator();

impl Instantiator {
    #[track_caller]
    pub fn instantiate(self, app: &mut App, registry: Registry) -> Addr {
        let endpoints = CwContractWrapper::new(execute::<()>, execute::<()>, registry.into());

        let code_id = app.store_code(Box::new(endpoints));

        app.instantiate(
            code_id,
            testing::user(ADMIN),
            &(),
            &[],
            "protocols_register",
            None,
        )
        .unwrap()
        .unwrap_response()
    }
}

pub(crate) fn no_protocols_query(
    deps: Deps<'_>,
    env: Env,
    msg: QueryMsg,
) -> ContractResult<Binary> {
    protocols_repo_query(0, deps, env, msg)
}

pub(crate) fn single_protocol_query(
    deps: Deps<'_>,
    env: Env,
    msg: QueryMsg,
) -> ContractResult<Binary> {
    protocols_repo_query(1, deps, env, msg)
}

pub(crate) fn two_protocols_query(
    deps: Deps<'_>,
    env: Env,
    msg: QueryMsg,
) -> ContractResult<Binary> {
    protocols_repo_query(2, deps, env, msg)
}

fn protocols_repo_query(
    protocols_nb: u8,
    deps: Deps<'_>,
    env: Env,
    msg: QueryMsg,
) -> ContractResult<Binary> {
    let res = match msg {
        QueryMsg::Protocols {} => to_json_binary::<ProtocolsQueryResponse>(
            &(0..protocols_nb).map(protocol_name).collect(),
        ),
        QueryMsg::Protocol(_) => {
            const NETWORK: Network = Network::Osmosis;
            const DEX: Dex = Dex::Osmosis;

            to_json_binary(&ProtocolQueryResponse {
                network: NETWORK,
                dex: DEX,
                contracts: ProtocolContractAddresses {
                    leaser: testing::contract(u64::MAX, u64::MAX),
                    lpp: testing::contract(2, 0),
                    oracle: testing::contract(4, 2),
                    profit: testing::contract(u64::MAX, u64::MAX),
                    reserve: testing::contract(u64::MAX, u64::MAX),
                },
            })
        }
        _ => Ok(dummy_query(deps, env, MockQueryMsg {})?),
    }?;

    Ok(res)
}

fn execute<Req>(
    _deps: DepsMut<'_, Empty>,
    _env: Env,
    _info: MessageInfo,
    _msg: Req,
) -> ContractResult<CwResponse> {
    Ok(Default::default())
}

fn protocol_name(index: u8) -> String {
    const PROTOCOL_NAME: &str = "my_nice_protocol";
    format!("{PROTOCOL_NAME}_{index}")
}
