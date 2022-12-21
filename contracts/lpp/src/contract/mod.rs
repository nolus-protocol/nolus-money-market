use serde::{de::DeserializeOwned, Serialize};

use access_control::SingleUserAccess;
use currency::lpn::Lpns;
use finance::currency::{visit_any_on_ticker, AnyVisitor, Currency};
#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo},
    cw2::set_contract_version,
};

use crate::{
    error::ContractError,
    lpp::LiquidityPool,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    state::Config,
};

mod borrow;
mod config;
mod lender;
mod rewards;

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

struct InstantiateWithLpn<'a> {
    deps: DepsMut<'a>,
    info: MessageInfo,
    msg: InstantiateMsg,
}

impl<'a> InstantiateWithLpn<'a> {
    // could be moved directly to on<LPN>()
    fn do_work<LPN>(self) -> Result<Response, ContractError>
    where
        LPN: 'static + Currency + Serialize + DeserializeOwned,
    {
        set_contract_version(self.deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

        SingleUserAccess::new(crate::access_control::OWNER_NAMESPACE, self.info.sender)
            .store(self.deps.storage)?;

        LiquidityPool::<LPN>::store(
            self.deps.storage,
            self.msg.lpn_ticker,
            self.msg.lease_code_id,
        )?;

        Ok(Response::new().add_attribute("method", "instantiate"))
    }

    pub fn cmd(
        deps: DepsMut<'a>,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> Result<Response, ContractError> {
        let context = Self { deps, info, msg };
        visit_any_on_ticker::<Lpns, _>(&context.msg.lpn_ticker.clone(), context)
    }
}

impl<'a> AnyVisitor for InstantiateWithLpn<'a> {
    type Output = Response;
    type Error = ContractError;

    fn on<LPN>(self) -> Result<Self::Output, Self::Error>
    where
        LPN: 'static + Currency + DeserializeOwned + Serialize,
    {
        self.do_work::<LPN>()
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    InstantiateWithLpn::cmd(deps, info, msg)
}

struct ExecuteWithLpn<'a> {
    deps: DepsMut<'a>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
}

impl<'a> ExecuteWithLpn<'a> {
    fn do_work<LPN>(self) -> Result<Response, ContractError>
    where
        LPN: 'static + Currency + Serialize + DeserializeOwned,
    {
        // currency context variants
        match self.msg {
            ExecuteMsg::OpenLoan { amount } => {
                let amount = amount.try_into()?;
                borrow::try_open_loan::<LPN>(self.deps, self.env, self.info, amount)
            }
            ExecuteMsg::RepayLoan() => {
                borrow::try_repay_loan::<LPN>(self.deps, self.env, self.info)
            }
            ExecuteMsg::Deposit() => lender::try_deposit::<LPN>(self.deps, self.env, self.info),
            ExecuteMsg::Burn { amount } => {
                lender::try_withdraw::<LPN>(self.deps, self.env, self.info, amount)
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
    ) -> Result<Response, ContractError> {
        let context = Self {
            deps,
            env,
            info,
            msg,
        };

        let config = Config::load(context.deps.storage)?;
        visit_any_on_ticker::<Lpns, _>(&config.lpn_ticker, context)
    }
}

impl<'a> AnyVisitor for ExecuteWithLpn<'a> {
    type Output = Response;
    type Error = ContractError;

    fn on<LPN>(self) -> Result<Self::Output, Self::Error>
    where
        LPN: 'static + Currency + DeserializeOwned + Serialize,
    {
        self.do_work::<LPN>()
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    // no currency context variants
    match msg {
        ExecuteMsg::UpdateParameters {
            base_interest_rate,
            utilization_optimal,
            addon_optimal_interest_rate,
        } => config::try_update_parameters(
            deps,
            info,
            base_interest_rate,
            utilization_optimal,
            addon_optimal_interest_rate,
        ),
        ExecuteMsg::DistributeRewards() => rewards::try_distribute_rewards(deps, info),
        ExecuteMsg::ClaimRewards { other_recipient } => {
            rewards::try_claim_rewards(deps, env, info, other_recipient)
        }
        _ => ExecuteWithLpn::cmd(deps, env, info, msg),
    }
}

struct QueryWithLpn<'a> {
    deps: Deps<'a>,
    env: Env,
    msg: QueryMsg,
}

impl<'a> QueryWithLpn<'a> {
    fn do_work<LPN>(self) -> Result<Binary, ContractError>
    where
        LPN: 'static + Currency + Serialize + DeserializeOwned,
    {
        // currency context variants
        let res = match self.msg {
            QueryMsg::Quote { amount } => {
                let quote = amount.try_into()?;
                to_binary(&borrow::query_quote::<LPN>(&self.deps, &self.env, quote)?)
            }
            QueryMsg::Loan { lease_addr } => to_binary(&borrow::query_loan::<LPN>(
                self.deps.storage,
                self.env,
                lease_addr,
            )?),
            QueryMsg::LoanOutstandingInterest {
                lease_addr,
                outstanding_time,
            } => to_binary(&borrow::query_loan_outstanding_interest::<LPN>(
                self.deps.storage,
                lease_addr,
                outstanding_time,
            )?),
            QueryMsg::LppBalance() => {
                to_binary(&rewards::query_lpp_balance::<LPN>(self.deps, self.env)?)
            }
            QueryMsg::Price() => {
                to_binary(&lender::query_ntoken_price::<LPN>(self.deps, self.env)?)
            }
            _ => {
                unreachable!()
            } // should be done already
        }?;
        Ok(res)
    }

    pub fn cmd(deps: Deps<'a>, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
        let context = Self { deps, env, msg };

        let config = Config::load(context.deps.storage)?;
        visit_any_on_ticker::<Lpns, _>(&config.lpn_ticker, context)
    }
}

impl<'a> AnyVisitor for QueryWithLpn<'a> {
    type Output = Binary;
    type Error = ContractError;

    fn on<LPN>(self) -> Result<Self::Output, Self::Error>
    where
        LPN: 'static + Currency + DeserializeOwned + Serialize,
    {
        self.do_work::<LPN>()
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
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
