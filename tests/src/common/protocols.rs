use admin_contract::{
    msg::{ProtocolContracts, ProtocolQueryResponse, ProtocolsQueryResponse, QueryMsg},
    result::Result as ContractResult,
};
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{to_json_binary, Addr, Binary, Deps, DepsMut, Empty, Env, MessageInfo},
};

use super::{dummy_query, test_case::app::App, CwContractWrapper, MockQueryMsg, ADMIN};

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
            Addr::unchecked(ADMIN),
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
        QueryMsg::Protocol { protocol: _ } => {
            const NET_NAME: &str = "dex_network";

            to_json_binary(&ProtocolQueryResponse {
                network: NET_NAME.into(),
                contracts: ProtocolContracts {
                    leaser: addr("DEADCODE"),
                    lpp: addr("contract0"),
                    oracle: addr("contract1"),
                    profit: addr("DEADCODE"),
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
    format!("{name}_{index}", name = PROTOCOL_NAME)
}

fn addr(raw_addr: &str) -> Addr {
    Addr::unchecked(raw_addr)
}
