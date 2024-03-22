use std::ops::DerefMut;

use access_control::SingleUserAccess;
use cosmwasm_std::{Addr, QuerierWrapper};
use currencies::Lpn as LpnCurrency;
use currency::Currency;
use finance::coin::Coin;
use platform::{
    bank::{self, BankAccount, BankAccountView},
    batch::Batch,
    contract::{self, Code},
    error as platform_error,
    message::Response as PlatformResponse,
    response,
};
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{self, entry_point, Binary, Deps, DepsMut, Env, MessageInfo},
};
use versioning::{package_version, version, SemVer, Version, VersionSegment};

use crate::{
    api::{ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    error::{Error, Result},
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
) -> Result<CwResponse> {
    deps.api
        .addr_validate(new_reserve.lease_code_admin.as_str())
        .map_err(Error::from)
        .and_then(|lease_code_admin| {
            SingleUserAccess::new(
                deps.storage.deref_mut(),
                crate::access_control::LEASE_CODE_ADMIN_KEY,
            )
            .grant_to(&lease_code_admin)
            .map_err(Into::into)
        })
        .and_then(|()| versioning::initialize(deps.storage, CONTRACT_VERSION).map_err(Into::into))
        .and_then(|()| {
            Code::try_new(new_reserve.lease_code_id.into(), &deps.querier).map_err(Into::into)
        })
        .and_then(|lease_code| Config::new(lease_code).store(deps.storage))
        .map(|()| response::empty_response())
        .or_else(|err| platform_error::log(err, deps.api))
}

#[entry_point]
pub fn migrate(deps: DepsMut<'_>, _env: Env, MigrateMsg {}: MigrateMsg) -> Result<CwResponse> {
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
) -> Result<CwResponse> {
    match msg {
        ExecuteMsg::NewLeaseCode(code) => SingleUserAccess::new(
            deps.storage.deref_mut(),
            crate::access_control::LEASE_CODE_ADMIN_KEY,
        )
        .check(&info.sender)
        .map_err(Into::into)
        .and_then(|()| Config::update_lease_code(deps.storage, code))
        .map(|()| PlatformResponse::default()),
        ExecuteMsg::CoverLiquidationLosses(amount) => {
            let lease = info.sender;
            Config::load(deps.storage)
                .and_then(|config| {
                    contract::validate_code_id(deps.querier, &lease, config.lease_code())
                        .map_err(Error::from)
                })
                .and_then(|()| amount.try_into().map_err(Into::into))
                .and_then(|losses| {
                    do_cover_losses(&lease, losses, &env.contract.address, deps.querier)
                })
        }
    }
    .map(response::response_only_messages)
    .or_else(|err| platform_error::log(err, deps.api))
}

#[entry_point]
pub fn query(deps: Deps<'_>, _env: Env, msg: QueryMsg) -> Result<Binary> {
    match msg {
        QueryMsg::Config() => Config::load(deps.storage)
            .map(ConfigResponse::from)
            .and_then(|config| cosmwasm_std::to_json_binary(&config).map_err(Into::into)),
        QueryMsg::ReserveLpn() => {
            cosmwasm_std::to_json_binary(&LpnCurrency::TICKER).map_err(Into::into)
        }
    }
    .map_err(Into::into)
    .or_else(|err| platform_error::log(err, deps.api))
}

fn do_cover_losses(
    lease: &Addr,
    amount: Coin<LpnCurrency>,
    this_contract: &Addr,
    querier: QuerierWrapper<'_>,
) -> Result<PlatformResponse> {
    let mut bank = bank::account(this_contract, querier);
    bank.balance::<LpnCurrency>()
        .map_err(Into::into)
        .and_then(|balance| {
            if balance < amount {
                Err(Error::InsufficientBalance)
            } else {
                bank.send(amount, lease);
                Ok(Batch::from(bank).into())
            }
        })
}
