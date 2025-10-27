use serde::Serialize;

use access_control::permissions::ProtocolAdminPermission;
use currencies::{
    Lpn as LpnCurrency, Lpns as LpnCurrencies, PaymentGroup, Stable as StableCurrency,
};
use currency::CurrencyDef;
use finance::coin::{Coin, CoinDTO};
use oracle::stub;
use oracle_platform::OracleRef;
use platform::{
    bank, contract::Code, error as platform_error, message::Response as PlatformResponse, response,
};
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, QuerierWrapper, entry_point},
};
use versioning::{
    ProtocolMigrationMessage, ProtocolPackageRelease, UpdatablePackage as _, VersionSegment,
    package_name, package_version,
};

use crate::{
    config::Config as ApiConfig,
    event::{self, WithdrawEmitter},
    lpp::{LiquidityPool, LppBalances},
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, SudoMsg},
    state::Config,
};

pub use self::error::{ContractError, Result};

mod borrow;
mod error;
mod lender;
mod rewards;

const CONTRACT_STORAGE_VERSION: VersionSegment = 3;
const CURRENT_RELEASE: ProtocolPackageRelease = ProtocolPackageRelease::current(
    package_name!(),
    package_version!(),
    CONTRACT_STORAGE_VERSION,
);

#[entry_point]
pub fn instantiate(
    mut deps: DepsMut<'_>,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<CwResponse> {
    let protocol_admin = deps.api.addr_validate(msg.protocol_admin.as_str())?;

    Code::try_new(
        msg.lease_code.into(),
        &platform::contract::validator(deps.querier),
    )
    .map_err(Into::into)
    .and_then(|lease_code| {
        let config = ApiConfig::new(
            lease_code,
            msg.borrow_rate,
            msg.min_utilization,
            protocol_admin,
        );
        Config::store(&config, deps.storage).map(|()| config)
    })
    .and_then(|ref config| {
        LiquidityPool::<LpnCurrency, _>::new(
            config,
            &bank::account_view(&env.contract.address, deps.querier),
        )
        .save(deps.storage)
    })
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
        .map_err(ContractError::UpdateSoftware)
        .inspect_err(platform_error::log(deps.api))
}

#[entry_point]
pub fn execute(
    mut deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg<LpnCurrencies>,
) -> Result<CwResponse> {
    let api = deps.api;
    match msg {
        ExecuteMsg::NewLeaseCode {
            lease_code: new_lease_code,
        } => {
            let loaded_config = Config::load(deps.storage)?;

            access_control::check(
                &ProtocolAdminPermission::new(loaded_config.protocol_admin()),
                &info,
            )?;

            Config::update_lease_code(deps.storage, new_lease_code)
                .map(|()| PlatformResponse::default())
                .map(response::response_only_messages)
        }
        ExecuteMsg::DistributeRewards() => Config::load(deps.storage)
            .and_then(|ref config| {
                rewards::try_distribute_rewards::<LpnCurrency, _>(
                    deps.storage,
                    info,
                    config,
                    &bank::account_view(&env.contract.address.clone(), deps.querier),
                )
            })
            .map(response::response_only_messages),
        ExecuteMsg::ClaimRewards { other_recipient } => {
            rewards::try_claim_rewards(deps, env, info, other_recipient)
                .map(response::response_only_messages)
        }
        ExecuteMsg::OpenLoan { amount } => amount
            .try_into()
            .map_err(Into::into)
            .and_then(|amount_lpn| {
                borrow::try_open_loan::<LpnCurrency>(deps, env, info, amount_lpn)
            })
            .and_then(|(loan_resp, message_response)| {
                response::response_with_messages::<_, _, ContractError>(loan_resp, message_response)
            }),
        ExecuteMsg::RepayLoan() => borrow::try_repay_loan::<LpnCurrency>(deps, env, info).and_then(
            |(excess_amount, message_response)| {
                response::response_with_messages::<_, _, ContractError>(
                    excess_amount,
                    message_response,
                )
            },
        ),
        ExecuteMsg::Deposit() => bank::received_one(&info.funds)
            .map_err(Into::into)
            .and_then(|pending_deposit| {
                lender::try_deposit::<LpnCurrency, _>(
                    deps.storage,
                    &bank::account_view(&env.contract.address.clone(), deps.querier),
                    info.sender.clone(),
                    pending_deposit,
                    &env.block.time,
                )
                .map(|receipts| {
                    PlatformResponse::from(event::emit_deposit(
                        env,
                        info.sender,
                        pending_deposit,
                        receipts,
                    ))
                })
            })
            .map(response::response_only_messages),
        ExecuteMsg::Burn { amount } => lender::try_withdraw::<LpnCurrency, _>(
            deps.storage,
            bank::account(&env.contract.address, deps.querier),
            info.sender.clone(),
            amount,
            &env.block.time,
            WithdrawEmitter::new(&env),
        )
        .map(response::response_only_messages),
        ExecuteMsg::CloseAllDeposits() => {
            let loaded_config = Config::load(deps.storage)?;

            access_control::check(
                &ProtocolAdminPermission::new(loaded_config.protocol_admin()),
                &info,
            )?;

            assert!(
                borrow::query_empty::<LpnCurrency>(deps.storage),
                "There is/are active loan(s)! The protocol admin should have checked it first!"
            );
            lender::try_close_all::<LpnCurrency, _, _>(
                deps.storage,
                bank::account_view(&env.contract.address.clone(), deps.querier),
                bank::account(&env.contract.address.clone(), deps.querier),
                &env.block.time,
                WithdrawEmitter::new(&env),
            )
            .map(response::response_only_messages)
        }
    }
    .inspect_err(platform_error::log(api))
}

#[entry_point]
pub fn sudo(deps: DepsMut<'_>, _env: Env, msg: SudoMsg) -> Result<CwResponse> {
    // no currency context variants
    match msg {
        SudoMsg::NewBorrowRate { borrow_rate } => {
            Config::update_borrow_rate(deps.storage, borrow_rate)
        }
        SudoMsg::MinUtilization { min_utilization } => {
            Config::update_min_utilization(deps.storage, min_utilization)
        }
    }
    .map(|()| PlatformResponse::default())
    .map(response::response_only_messages)
    .inspect_err(platform_error::log(deps.api))
}

#[entry_point]
pub fn query(deps: Deps<'_>, env: Env, msg: QueryMsg<LpnCurrencies>) -> Result<Binary> {
    match msg {
        QueryMsg::Config() => Config::load(deps.storage).and_then(|ref resp| to_json_binary(resp)),
        QueryMsg::ProtocolPackageRelease {} => to_json_binary(&CURRENT_RELEASE),
        QueryMsg::Lpn() => to_json_binary(LpnCurrency::dto()),
        QueryMsg::Balance { address } => {
            lender::query_balance(deps.storage, address).and_then(|ref resp| to_json_binary(resp))
        }
        QueryMsg::Rewards { address } => {
            rewards::query_rewards(deps.storage, address).and_then(|ref resp| to_json_binary(resp))
        }
        QueryMsg::Quote { amount } => amount
            .try_into()
            .map_err(Into::into)
            .and_then(|quote| borrow::query_quote::<LpnCurrency>(&deps, &env, quote))
            .and_then(|ref resp| to_json_binary(resp)),
        QueryMsg::Loan { lease_addr } => {
            borrow::query_loan::<LpnCurrency>(deps.storage, lease_addr)
                .and_then(|ref resp| to_json_binary(resp))
        }
        QueryMsg::LppBalance() => {
            let bank = bank::account_view(&env.contract.address, deps.querier);
            Config::load(deps.storage)
                .and_then(|ref config| {
                    rewards::query_lpp_balance::<LpnCurrency, _>(
                        deps.storage,
                        config,
                        &bank,
                        &env.block.time,
                    )
                    .and_then(|lpp_balances| {
                        rewards::query_total_receipts::<LpnCurrency, _>(deps.storage, config, &bank)
                            .map(|total_receipts| lpp_balances.into_response(total_receipts))
                    })
                })
                .and_then(|ref resp| to_json_binary(resp))
        }
        QueryMsg::StableBalance { oracle_addr } => Config::load(deps.storage)
            .and_then(|ref config| {
                rewards::query_lpp_balance::<LpnCurrency, _>(
                    deps.storage,
                    config,
                    &bank::account_view(&env.contract.address, deps.querier),
                    &env.block.time,
                )
            })
            .map(LppBalances::into_total)
            .and_then(|total| {
                OracleRef::try_from_base(oracle_addr, deps.querier)
                    .map_err(ContractError::InvalidOracleBaseCurrency)
                    .and_then(|oracle_ref| to_stable(oracle_ref, total, deps.querier))
            })
            .map(CoinDTO::<PaymentGroup>::from)
            .and_then(|ref resp| to_json_binary(resp)),
        QueryMsg::Price() => lender::query_ntoken_price::<LpnCurrency, _>(
            deps.storage,
            &bank::account_view(&env.contract.address, deps.querier),
            &env.block.time,
        )
        .and_then(|ref resp| to_json_binary(resp)),
        QueryMsg::DepositCapacity() => to_json_binary(&lender::deposit_capacity::<LpnCurrency, _>(
            deps.storage,
            &bank::account_view(&env.contract.address, deps.querier),
            &env.block.time,
        )?),
    }
    .inspect_err(platform_error::log(deps.api))
}

fn to_json_binary<T>(data: &T) -> Result<Binary>
where
    T: Serialize,
{
    cosmwasm_std::to_json_binary(data).map_err(ContractError::ConvertToBinary)
}

fn to_stable(
    oracle: OracleRef<LpnCurrency, LpnCurrencies>,
    total: Coin<LpnCurrency>,
    querier: QuerierWrapper<'_>,
) -> Result<Coin<StableCurrency>> {
    stub::from_quote::<_, LpnCurrencies, StableCurrency, PaymentGroup>(oracle, total, querier)
        .map_err(ContractError::ConvertFromQuote)
}

#[cfg(test)]
mod test {
    use currencies::Lpn;
    use sdk::cosmwasm_std::{Addr, MessageInfo};

    pub(super) type TheCurrency = Lpn;

    pub(super) fn lender() -> Addr {
        const LENDER: &str = "lender";

        Addr::unchecked(LENDER)
    }

    pub(super) fn lender_msg_no_funds() -> MessageInfo {
        MessageInfo {
            sender: lender(),
            funds: vec![],
        }
    }
}
