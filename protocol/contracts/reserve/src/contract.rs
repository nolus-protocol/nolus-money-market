use cosmwasm_std::Storage;

use access_control::permissions::ProtocolAdminPermission;
use currencies::Lpn as LpnCurrency;
use currency::CurrencyDef;
use finance::coin::Coin;
use platform::{
    bank::{self, BankAccount, BankAccountView},
    batch::{Emit, Emitter},
    contract::{self, Code, Validator},
    error as platform_error,
    message::Response as PlatformResponse,
    response,
};
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{
        self, Addr, Binary, Deps, DepsMut, Env, MessageInfo, QuerierWrapper, entry_point,
    },
};
use versioning::{
    ProtocolMigrationMessage, ProtocolPackageRelease, UpdatablePackage as _, VersionSegment,
    package_name, package_version,
};

use crate::{
    api::{ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    error::{Error, Result},
    state::Config,
};

const CONTRACT_STORAGE_VERSION: VersionSegment = 0;
const CURRENT_RELEASE: ProtocolPackageRelease = ProtocolPackageRelease::current(
    package_name!(),
    package_version!(),
    CONTRACT_STORAGE_VERSION,
);

#[entry_point]
pub fn instantiate(
    mut deps: DepsMut<'_>,
    _env: Env,
    _info: MessageInfo,
    new_reserve: InstantiateMsg,
) -> Result<CwResponse> {
    let lease_code_admin = deps
        .api
        .addr_validate(new_reserve.lease_code_admin.as_str())?;

    Code::try_new(
        new_reserve.lease_code.into(),
        &platform::contract::validator(deps.querier),
    )
    .map_err(Into::into)
    .and_then(|lease_code| Config::new(lease_code, lease_code_admin).store(deps.storage))
    .map(|()| response::empty_response())
    .inspect_err(platform_error::log(deps.api))
}

#[entry_point]
pub fn migrate(
    deps: DepsMut<'_>,
    _env: Env,
    ProtocolMigrationMessage {
        migrate_from,
        to_release,
        message: MigrateMsg {},
    }: ProtocolMigrationMessage<MigrateMsg>,
) -> Result<CwResponse> {
    migrate_from
        .update_software(&CURRENT_RELEASE, &to_release)
        .map(|()| response::empty_response())
        .map_err(Error::UpdateSoftware)
        .inspect_err(platform_error::log(deps.api))
}

#[entry_point]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<CwResponse> {
    let cfg = Config::load(deps.storage)?;
    let lease_code_admin = cfg.lease_code_admin();

    match msg {
        ExecuteMsg::NewLeaseCode(code) => {
            access_control::check(&ProtocolAdminPermission::new(&lease_code_admin), &info)
                .map_err(Into::into)
                .and_then(|()| Config::update_lease_code(deps.storage, code)) // TODO - reuse cfg
                .map(|()| PlatformResponse::default())
        }
        ExecuteMsg::CoverLiquidationLosses(amount) => contract::validator(deps.querier)
            .check_contract_code(info.sender, &cfg.lease_code())
            .map_err(Error::from)
            .and_then(|lease| {
                amount.try_into().map_err(Into::into).and_then(|losses| {
                    do_cover_losses(lease, losses, &env.contract.address, deps.querier)
                })
            }),
        ExecuteMsg::DumpBalanceTo(receiver) => {
            authorize_protocol_admin_only(deps.storage.deref(), &info)
                .and_then(|()| dump_balance_to(&env.contract.address, receiver, deps.querier))
        }
    }
    .map(response::response_only_messages)
    .inspect_err(platform_error::log(deps.api))
}

#[entry_point]
pub fn query(deps: Deps<'_>, _env: Env, msg: QueryMsg) -> Result<Binary> {
    match msg {
        QueryMsg::ReserveLpn() => {
            cosmwasm_std::to_json_binary(&currency::to_string(LpnCurrency::dto()))
                .map_err(Into::into)
        }
        QueryMsg::Config() => Config::load(deps.storage)
            .map(ConfigResponse::from)
            .and_then(|config| cosmwasm_std::to_json_binary(&config).map_err(Into::into)),
        QueryMsg::ProtocolPackageRelease {} => {
            cosmwasm_std::to_json_binary(&CURRENT_RELEASE).map_err(Into::into)
        }
    }
    .inspect_err(platform_error::log(deps.api))
}

fn authorize_protocol_admin_only(store: &dyn Storage, call_message: &MessageInfo) -> Result<()> {
    SingleUserAccess::new(store, crate::access_control::PROTOCOL_ADMIN_KEY)
        .check(call_message)
        .map_err(Into::into)
}

fn do_cover_losses(
    lease: Addr,
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
                bank.send(amount, lease.clone());
                let emitter = Emitter::of_type("reserve-cover-loss")
                    .emit("to", lease)
                    .emit_coin("payment", amount);

                Ok(PlatformResponse::messages_with_event(bank.into(), emitter))
            }
        })
}

fn dump_balance_to(
    reserve: &Addr,
    receiver: Addr,
    querier: QuerierWrapper<'_>,
) -> Result<PlatformResponse> {
    let mut reserve_account = bank::account(reserve, querier);
    reserve_account
        .balance::<LpnCurrency>()
        .map_err(Error::ObtainBalance)
        .map(|balance| {
            if !balance.is_zero() {
                reserve_account.send(balance, receiver);
            }
            PlatformResponse::messages_only(reserve_account.into())
        })
}
