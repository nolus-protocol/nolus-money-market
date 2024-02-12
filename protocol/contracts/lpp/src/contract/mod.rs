use std::ops::DerefMut as _;

use serde::{de::DeserializeOwned, Serialize};

use access_control::SingleUserAccess;
use currency::{AnyVisitor, AnyVisitorResult, Currency, GroupVisit, Tickers};
use platform::{message::Response as PlatformResponse, response};
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo},
};
use versioning::{package_version, version, SemVer, Version, VersionSegment};

use crate::{
    error::{ContractError, Result},
    lpp::{LiquidityPool, LpnCurrencies},
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, SudoMsg},
    state::Config,
};

mod borrow;
mod lender;
mod rewards;

const CONTRACT_STORAGE_VERSION: VersionSegment = 1;
const PACKAGE_VERSION: SemVer = package_version!();
const CONTRACT_VERSION: Version = version!(CONTRACT_STORAGE_VERSION, PACKAGE_VERSION);

struct InstantiateWithLpn<'a> {
    deps: DepsMut<'a>,
    msg: InstantiateMsg,
}

impl<'a> InstantiateWithLpn<'a> {
    // could be moved directly to on<LPN>()
    fn do_work<Lpn>(mut self) -> Result<()>
    where
        Lpn: 'static + Currency + Serialize + DeserializeOwned,
    {
        versioning::initialize(self.deps.storage, CONTRACT_VERSION)?;

        SingleUserAccess::new(
            self.deps.storage.deref_mut(),
            crate::access_control::LEASE_CODE_ADMIN_KEY,
        )
        .grant_to(&self.msg.lease_code_admin)?;

        LiquidityPool::<Lpn>::store(self.deps.storage, self.msg.into())
    }

    pub fn cmd(deps: DepsMut<'a>, msg: InstantiateMsg) -> Result<()> {
        let context = Self { deps, msg };

        Tickers.visit_any::<LpnCurrencies, _>(&context.msg.lpn_ticker.clone(), context)
    }
}

impl<'a> AnyVisitor for InstantiateWithLpn<'a> {
    type Output = ();
    type Error = ContractError;

    fn on<Lpn>(self) -> AnyVisitorResult<Self>
    where
        Lpn: 'static + Currency + DeserializeOwned + Serialize,
    {
        self.do_work::<Lpn>()
    }
}

#[entry_point]
pub fn instantiate(
    deps: DepsMut<'_>,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<CwResponse> {
    // TODO move these checks on deserialization
    currency::validate::<LpnCurrencies>(&msg.lpn_ticker)?;
    deps.api.addr_validate(msg.lease_code_admin.as_str())?;

    InstantiateWithLpn::cmd(deps, msg).map(|()| response::empty_response())
}

#[entry_point]
pub fn migrate(deps: DepsMut<'_>, _env: Env, msg: MigrateMsg) -> Result<CwResponse> {
    // Statically assert that the message is empty when doing a software-only update.
    let MigrateMsg {}: MigrateMsg = msg;

    versioning::update_software(deps.storage, CONTRACT_VERSION, Into::into)
        .and_then(response::response)
}

struct ExecuteWithLpn<'a> {
    deps: DepsMut<'a>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg<LpnCurrencies>,
}

impl<'a> ExecuteWithLpn<'a> {
    fn do_work<Lpn>(self) -> Result<CwResponse>
    where
        Lpn: 'static + Currency + Serialize + DeserializeOwned,
    {
        // currency context variants
        match self.msg {
            ExecuteMsg::OpenLoan { amount } => amount
                .try_into()
                .map_err(Into::into)
                .and_then(|amount_lpn| {
                    borrow::try_open_loan::<Lpn>(self.deps, self.env, self.info, amount_lpn)
                })
                .and_then(|(loan_resp, message_response)| {
                    response::response_with_messages::<_, _, ContractError>(
                        loan_resp,
                        message_response,
                    )
                }),
            ExecuteMsg::RepayLoan() => borrow::try_repay_loan::<Lpn>(
                self.deps, self.env, self.info,
            )
            .and_then(|(excess_amount, message_response)| {
                response::response_with_messages::<_, _, ContractError>(
                    excess_amount,
                    message_response,
                )
            }),
            ExecuteMsg::Deposit() => lender::try_deposit::<Lpn>(self.deps, self.env, self.info)
                .map(response::response_only_messages),
            ExecuteMsg::Burn { amount } => {
                lender::try_withdraw::<Lpn>(self.deps, self.env, self.info, amount)
                    .map(response::response_only_messages)
            }
            _ => {
                unreachable!()
            } // should be done already
        }
    }

    pub fn cmd(
        deps: DepsMut<'a>,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg<LpnCurrencies>,
    ) -> Result<CwResponse> {
        let context = Self {
            deps,
            env,
            info,
            msg,
        };

        let config = Config::load(context.deps.storage)?;

        Tickers.visit_any::<LpnCurrencies, _>(config.lpn_ticker(), context)
    }
}

impl<'a> AnyVisitor for ExecuteWithLpn<'a> {
    type Output = CwResponse;
    type Error = ContractError;

    fn on<Lpn>(self) -> AnyVisitorResult<Self>
    where
        Lpn: 'static + Currency + DeserializeOwned + Serialize,
    {
        self.do_work::<Lpn>()
    }
}

#[entry_point]
pub fn execute(
    mut deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg<LpnCurrencies>,
) -> Result<CwResponse> {
    // no currency context variants
    match msg {
        ExecuteMsg::NewLeaseCode { lease_code_id } => {
            SingleUserAccess::new(
                deps.storage.deref_mut(),
                crate::access_control::LEASE_CODE_ADMIN_KEY,
            )
            .check(&info.sender)?;

            Config::update_lease_code(deps.storage, lease_code_id)
                .map(|()| PlatformResponse::default())
                .map(response::response_only_messages)
        }
        ExecuteMsg::DistributeRewards() => {
            rewards::try_distribute_rewards(deps, info).map(response::response_only_messages)
        }
        ExecuteMsg::ClaimRewards { other_recipient } => {
            rewards::try_claim_rewards(deps, env, info, other_recipient)
                .map(response::response_only_messages)
        }
        _ => ExecuteWithLpn::cmd(deps, env, info, msg),
    }
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
}

struct QueryWithLpn<'a> {
    deps: Deps<'a>,
    env: Env,
    msg: QueryMsg<LpnCurrencies>,
}

impl<'a> QueryWithLpn<'a> {
    fn do_work<Lpn>(self) -> Result<Binary>
    where
        Lpn: 'static + Currency + Serialize + DeserializeOwned,
    {
        // currency context variants
        let res = match self.msg {
            QueryMsg::Quote { amount } => {
                let quote = amount.try_into()?;

                to_json_binary(&borrow::query_quote::<Lpn>(&self.deps, &self.env, quote)?)
            }
            QueryMsg::Loan { lease_addr } => {
                to_json_binary(&borrow::query_loan::<Lpn>(self.deps.storage, lease_addr)?)
            }
            QueryMsg::LppBalance() => {
                to_json_binary(&rewards::query_lpp_balance::<Lpn>(self.deps, self.env)?)
            }
            QueryMsg::Price() => {
                to_json_binary(&lender::query_ntoken_price::<Lpn>(self.deps, self.env)?)
            }
            QueryMsg::DepositCapacity() => {
                to_json_binary(&lender::deposit_capacity::<Lpn>(self.deps, self.env)?)
            }
            _ => unreachable!("Variants should have been exhausted!"),
        }?;

        Ok(res)
    }

    pub fn cmd(deps: Deps<'a>, env: Env, msg: QueryMsg<LpnCurrencies>) -> Result<Binary> {
        let context = Self { deps, env, msg };

        let config = Config::load(context.deps.storage)?;

        Tickers.visit_any::<LpnCurrencies, _>(config.lpn_ticker(), context)
    }
}

impl<'a> AnyVisitor for QueryWithLpn<'a> {
    type Output = Binary;
    type Error = ContractError;

    fn on<Lpn>(self) -> AnyVisitorResult<Self>
    where
        Lpn: 'static + Currency + DeserializeOwned + Serialize,
    {
        self.do_work::<Lpn>()
    }
}

#[entry_point]
pub fn query(deps: Deps<'_>, env: Env, msg: QueryMsg<LpnCurrencies>) -> Result<Binary> {
    match msg {
        QueryMsg::Config() => to_json_binary(&Config::load(deps.storage)?).map_err(Into::into),
        QueryMsg::Balance { address } => {
            to_json_binary(&lender::query_balance(deps.storage, address)?).map_err(Into::into)
        }
        QueryMsg::Rewards { address } => {
            to_json_binary(&rewards::query_rewards(deps.storage, address)?).map_err(Into::into)
        }
        _ => QueryWithLpn::cmd(deps, env, msg),
    }
}
