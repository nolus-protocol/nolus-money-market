use std::ops::DerefMut;

use access_control::SingleUserAccess;
use finance::coin::Coin;
use platform::{
    bank::{self, BankAccount, BankAccountView},
    batch::Batch,
    contract, error as platform_error,
    message::Response as PlatformResponse,
    response,
};
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{self, entry_point, Binary, Deps, DepsMut, Env, MessageInfo},
};
use versioning::{package_version, version, SemVer, Version, VersionSegment};

use crate::{
    api::{ConfigResponse, ExecuteMsg, InstantiateMsg, LpnCurrency, MigrateMsg, QueryMsg},
    error::{ContractError, ContractResult},
    state::Config,
};

// const CONTRACT_STORAGE_VERSION_FROM: VersionSegment = 0;
const CONTRACT_STORAGE_VERSION: VersionSegment = 0;
const PACKAGE_VERSION: SemVer = package_version!();
const CONTRACT_VERSION: Version = version!(CONTRACT_STORAGE_VERSION, PACKAGE_VERSION);

#[entry_point]
pub fn instantiate(
    mut deps: DepsMut<'_>,
    _env: Env,
    _info: MessageInfo,
    new_reserve: InstantiateMsg,
) -> ContractResult<CwResponse> {
    deps.api
        .addr_validate(new_reserve.lease_code_admin.as_str())
        .map_err(Into::<ContractError>::into)
        .and_then(|_| {
            SingleUserAccess::new(
                deps.storage.deref_mut(),
                crate::access_control::LEASE_CODE_ADMIN_KEY,
            )
            .grant_to(&new_reserve.lease_code_admin)
            .map_err(Into::into)
        })
        .and_then(|()| versioning::initialize(deps.storage, CONTRACT_VERSION).map_err(Into::into))
        .and_then(|()| Config::from(new_reserve).store(deps.storage))
        .map(|()| response::empty_response())
        .or_else(|err| platform_error::log(err, deps.api))
}

#[entry_point]
pub fn migrate(deps: DepsMut<'_>, __env: Env, _msg: MigrateMsg) -> ContractResult<CwResponse> {
    versioning::update_software(deps.storage, CONTRACT_VERSION, Into::into)
        .and_then(response::response)
        .or_else(|err| platform_error::log(err, deps.api))
}

#[entry_point]
pub fn execute(
    mut deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<CwResponse> {
    match msg {
        ExecuteMsg::NewLeaseCode {
            code_id: new_code_id,
        } => {
            SingleUserAccess::new(
                deps.storage.deref_mut(),
                crate::access_control::LEASE_CODE_ADMIN_KEY,
            )
            .check(&info.sender)?;

            Config::update_lease_code(deps.storage, new_code_id.into())
                .map(|()| PlatformResponse::default())
        }
        ExecuteMsg::CoverLiquidationLosses { amount } => {
            let lease = info.sender;

            Config::load(deps.storage)
                .and_then(|config| {
                    contract::validate_code_id(deps.querier, &lease, config.lease_code_id())
                        .map_err(ContractError::from)
                })
                .and_then(|()| amount.try_into().map_err(Into::into))
                .and_then(|amount: Coin<LpnCurrency>| {
                    let mut bank = bank::account(&env.contract.address, deps.querier);
                    bank.balance::<LpnCurrency>()
                        .map_err(Into::into)
                        .and_then(|balance| {
                            if balance < amount {
                                Err(ContractError::InsufficientBalance)
                            } else {
                                bank.send(amount, &lease);
                                let msg: Batch = bank.into();

                                Ok(msg.into())
                            }
                        })
                })
        }
    }
    .map(response::response_only_messages)
    .or_else(|err| platform_error::log(err, deps.api))
}

#[entry_point]
pub fn query(deps: Deps<'_>, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::Config() => Config::load(deps.storage)
            .map(Into::<ConfigResponse>::into)
            .and_then(|config| cosmwasm_std::to_json_binary(&config).map_err(Into::into))
            .or_else(|err| platform_error::log(err, deps.api)),
    }
}
