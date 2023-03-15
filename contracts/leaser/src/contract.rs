use access_control::SingleUserAccess;
use platform::{batch::Batch, reply::from_instantiate, response};
#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply},
};
use versioning::{version, VersionSegment};

use crate::{
    cmd::Borrow,
    error::{ContractError, ContractResult},
    leaser::{self, Leaser},
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, SudoMsg},
    state::{config::Config, leases::Leases},
};

// version info for migration info
// const CONTRACT_STORAGE_VERSION_FROM: VersionSegment = 0;
const CONTRACT_STORAGE_VERSION: VersionSegment = 0;

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn instantiate(
    deps: DepsMut<'_>,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    platform::contract::validate_addr(&deps.querier, &msg.lpp_ust_addr)?;
    platform::contract::validate_addr(&deps.querier, &msg.time_alarms)?;
    platform::contract::validate_addr(&deps.querier, &msg.market_price_oracle)?;
    platform::contract::validate_addr(&deps.querier, &msg.profit)?;

    versioning::initialize(deps.storage, version!(CONTRACT_STORAGE_VERSION))?;

    SingleUserAccess::new_contract_owner(info.sender).store(deps.storage)?;

    let lease_code = msg.lease_code_id;
    Config::new(msg)?.store(deps.storage)?;
    // require the config to be stored before
    let mut batch = Batch::default();
    leaser::update_lpp(deps.storage, lease_code.u64(), &mut batch)?;

    Ok(batch.into())
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn migrate(deps: DepsMut<'_>, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    versioning::update_software(deps.storage, version!(CONTRACT_STORAGE_VERSION))?;

    response::response(versioning::release()).map_err(Into::into)
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::OpenLease { currency, max_ltv } => Borrow::with(
            deps,
            info.funds,
            info.sender,
            env.contract.address,
            currency,
            max_ltv,
        ),
        ExecuteMsg::Sudo { msg } => {
            SingleUserAccess::check_owner_access(deps.storage, &info.sender)
                .and_then(move |()| sudo(deps, env, msg))
        }
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn sudo(deps: DepsMut<'_>, _env: Env, msg: SudoMsg) -> ContractResult<Response> {
    match msg {
        SudoMsg::SetupDex(params) => leaser::try_setup_dex(deps.storage, params),
        SudoMsg::Config {
            lease_interest_rate_margin,
            liability,
            lease_interest_payment,
        } => leaser::try_configure(
            deps.storage,
            lease_interest_rate_margin,
            liability,
            lease_interest_payment,
        ),
        SudoMsg::MigrateLeases { new_code_id } => {
            leaser::try_migrate_leases(deps.storage, new_code_id.u64())
        }
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn query(deps: Deps<'_>, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    let res = match msg {
        QueryMsg::Config {} => to_binary(&Leaser::new(deps).config()?),
        QueryMsg::Quote {
            downpayment,
            lease_asset,
            max_ltv,
        } => to_binary(&Leaser::new(deps).quote(downpayment, lease_asset, max_ltv)?),
        QueryMsg::Leases { owner } => to_binary(&Leaser::new(deps).customer_leases(owner)?),
    };
    res.map_err(ContractError::from)
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn reply(deps: DepsMut<'_>, _env: Env, msg: Reply) -> ContractResult<Response> {
    let msg_id = msg.id;
    let contract_addr = from_instantiate::<()>(deps.api, msg)
        .map(|r| r.address)
        .map_err(|err| ContractError::ParseError {
            err: err.to_string(),
        })?;

    Leases::save(deps.storage, msg_id, contract_addr.clone())?;
    Ok(Response::new().add_attribute("lease_address", contract_addr))
}
