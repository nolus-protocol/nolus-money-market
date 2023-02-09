use access_control::SingleUserAccess;
use platform::{batch::Batch, reply::from_instantiate};
#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Storage},
};
use versioning::{version, VersionSegment};

use crate::{
    cmd::Borrow,
    error::{ContractError, ContractResult},
    leaser::{self, Leaser},
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
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
    versioning::upgrade_old_contract::<1, fn(_) -> _, ContractError>(
        deps.storage,
        version!(CONTRACT_STORAGE_VERSION),
        None,
    )?;

    Ok(Response::default())
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::SetupDex(params) => {
            owner_allowed_only(deps.storage, info, |s| leaser::try_setup_dex(s, params))
        }
        ExecuteMsg::Config {
            lease_interest_rate_margin,
            liability,
            lease_interest_payment,
        } => owner_allowed_only(deps.storage, info, |s| {
            leaser::try_configure(
                s,
                lease_interest_rate_margin,
                liability,
                lease_interest_payment,
            )
        }),
        ExecuteMsg::MigrateLeases { new_code_id } => owner_allowed_only(deps.storage, info, |s| {
            leaser::try_migrate_leases(s, new_code_id.u64())
        }),
        ExecuteMsg::OpenLease { currency } => Borrow::with(
            deps,
            info.funds,
            info.sender,
            env.contract.address,
            currency,
        ),
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn query(deps: Deps<'_>, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    let res = match msg {
        QueryMsg::Config {} => to_binary(&Leaser::new(deps).config()?),
        QueryMsg::Quote {
            downpayment,
            lease_asset,
        } => to_binary(&Leaser::new(deps).quote(downpayment, lease_asset)?),
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

fn owner_allowed_only<'a, F, R>(
    storage: &'a mut dyn Storage,
    info: MessageInfo,
    restricted: F,
) -> ContractResult<R>
where
    F: FnOnce(&'a mut dyn Storage) -> ContractResult<R>,
{
    SingleUserAccess::check_owner_access::<ContractError>(storage, &info.sender)?;
    restricted(storage)
}

#[cfg(test)]
mod test {

    use access_control::SingleUserAccess;
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_info},
        Addr,
    };

    use crate::ContractError;

    #[test]
    fn exec_by_non_owner() {
        let owner = Addr::unchecked("the big boss");
        let caller = "bad boy";
        let mut deps = mock_dependencies();
        SingleUserAccess::new_contract_owner(owner)
            .store(&mut deps.storage)
            .unwrap();

        let err = super::owner_allowed_only::<_, bool>(
            &mut deps.storage,
            mock_info(caller, &[]),
            |_| unreachable!(),
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized(_)))
    }

    #[test]
    fn exec_by_owner() {
        let owner = Addr::unchecked("the big boss");
        let mut deps = mock_dependencies();
        SingleUserAccess::new_contract_owner(owner.clone())
            .store(&mut deps.storage)
            .unwrap();

        let res =
            super::owner_allowed_only(&mut deps.storage, mock_info(owner.as_str(), &[]), |_| {
                Ok(true)
            })
            .unwrap();
        assert!(res);
    }
}
