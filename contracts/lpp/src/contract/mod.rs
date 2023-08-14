use std::ops::DerefMut;

use serde::{de::DeserializeOwned, Serialize};

use access_control::SingleUserAccess;
use currency::{lpn::Lpns, AnyVisitor, AnyVisitorResult, Currency};
use platform::response;
#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Storage},
};
use versioning::{version, VersionSegment};

use crate::{
    error::{ContractError, Result},
    lpp::LiquidityPool,
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, SudoMsg},
    state::Config,
};

mod borrow;
mod config;
mod lender;
mod migrate;
mod rewards;

// version info for migration info
const CONTRACT_STORAGE_VERSION_FROM: VersionSegment = 0;
const CONTRACT_STORAGE_VERSION: VersionSegment = 1;

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
        versioning::initialize(self.deps.storage, version!(CONTRACT_STORAGE_VERSION))?;

        SingleUserAccess::new(
            self.deps.storage.deref_mut(),
            crate::access_control::LEASE_CODE_ADMIN_KEY,
        )
        .grant_to(&self.msg.lease_code_admin)?;

        LiquidityPool::<Lpn>::store(self.deps.storage, self.msg.into())
    }

    pub fn cmd(deps: DepsMut<'a>, msg: InstantiateMsg) -> Result<()> {
        let context = Self { deps, msg };

        currency::visit_any_on_ticker::<Lpns, _>(&context.msg.lpn_ticker.clone(), context)
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

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn instantiate(
    deps: DepsMut<'_>,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<CwResponse> {
    // TODO move these checks on deserialization
    currency::validate::<Lpns>(&msg.lpn_ticker)?;
    deps.api.addr_validate(msg.lease_code_admin.as_str())?;

    InstantiateWithLpn::cmd(deps, msg).map(|()| response::empty_response())
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn migrate(deps: DepsMut<'_>, _env: Env, msg: MigrateMsg) -> Result<CwResponse> {
    versioning::update_software_and_storage::<CONTRACT_STORAGE_VERSION_FROM, _, _, _, _>(
        deps.storage,
        version!(CONTRACT_STORAGE_VERSION),
        |storage: &mut dyn Storage| self::migrate::migrate(storage, msg.min_utilization),
        Into::into,
    )
    .and_then(|(label, ())| response::response(label))
}

struct ExecuteWithLpn<'a> {
    deps: DepsMut<'a>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
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
                        &loan_resp,
                        message_response,
                    )
                }),
            ExecuteMsg::RepayLoan() => borrow::try_repay_loan::<Lpn>(
                self.deps, self.env, self.info,
            )
            .and_then(|(excess_amount, message_response)| {
                response::response_with_messages::<_, _, ContractError>(
                    &excess_amount,
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
        msg: ExecuteMsg,
    ) -> Result<CwResponse> {
        let context = Self {
            deps,
            env,
            info,
            msg,
        };

        let config = Config::load(context.deps.storage)?;

        currency::visit_any_on_ticker::<Lpns, _>(config.lpn_ticker(), context)
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

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<CwResponse> {
    // no currency context variants
    match msg {
        ExecuteMsg::NewLeaseCode { lease_code_id } => {
            config::try_update_lease_code(deps, info, lease_code_id)
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

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn sudo(deps: DepsMut<'_>, _env: Env, msg: SudoMsg) -> Result<CwResponse> {
    // no currency context variants
    match msg {
        SudoMsg::UpdateParameters {
            borrow_rate,
            min_utilization,
        } => config::try_update_parameters(deps, borrow_rate, min_utilization)
            .map(response::response_only_messages),
    }
}

struct QueryWithLpn<'a> {
    deps: Deps<'a>,
    env: Env,
    msg: QueryMsg,
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

                to_binary(&borrow::query_quote::<Lpn>(&self.deps, &self.env, quote)?)
            }
            QueryMsg::Loan { lease_addr } => {
                to_binary(&borrow::query_loan::<Lpn>(self.deps.storage, lease_addr)?)
            }
            QueryMsg::LppBalance() => {
                to_binary(&rewards::query_lpp_balance::<Lpn>(self.deps, self.env)?)
            }
            QueryMsg::Price() => {
                to_binary(&lender::query_ntoken_price::<Lpn>(self.deps, self.env)?)
            }
            QueryMsg::DepositLimit() => {
                to_binary(&lender::deposit_limit::<Lpn>(self.deps, self.env)?)
            }
            _ => unreachable!("Variants should have been exhausted!"),
        }?;

        Ok(res)
    }

    pub fn cmd(deps: Deps<'a>, env: Env, msg: QueryMsg) -> Result<Binary> {
        let context = Self { deps, env, msg };

        let config = Config::load(context.deps.storage)?;

        currency::visit_any_on_ticker::<Lpns, _>(config.lpn_ticker(), context)
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

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn query(deps: Deps<'_>, env: Env, msg: QueryMsg) -> Result<Binary> {
    let res = match msg {
        QueryMsg::Config() => to_binary(&config::query_config(&deps)?)?,
        QueryMsg::Balance { address } => to_binary(&lender::query_balance(deps.storage, address)?)?,
        QueryMsg::Rewards { address } => {
            to_binary(&rewards::query_rewards(deps.storage, address)?)?
        }
        _ => QueryWithLpn::cmd(deps, env, msg)?,
    };

    Ok(res)
}
