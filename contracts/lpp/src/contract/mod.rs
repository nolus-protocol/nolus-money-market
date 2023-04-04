use serde::{de::DeserializeOwned, Serialize};

use access_control::SingleUserAccess;
use currency::lpn::Lpns;
use finance::currency::{visit_any_on_ticker, AnyVisitor, AnyVisitorResult, Currency};
use platform::response;
#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, StdResult},
};
use versioning::{version, VersionSegment};

use crate::{
    error::{ContractError, ContractResult},
    lpp::LiquidityPool,
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, SudoMsg},
    state::Config,
};

mod borrow;
mod config;
mod lender;
mod rewards;

// version info for migration info
// const CONTRACT_STORAGE_VERSION_FROM: VersionSegment = 0;
const CONTRACT_STORAGE_VERSION: VersionSegment = 0;

struct InstantiateWithLpn<'a> {
    deps: DepsMut<'a>,
    msg: InstantiateMsg,
}

impl<'a> InstantiateWithLpn<'a> {
    // could be moved directly to on<LPN>()
    fn do_work<LPN>(self) -> ContractResult<Response>
    where
        LPN: 'static + Currency + Serialize + DeserializeOwned,
    {
        versioning::initialize(self.deps.storage, version!(CONTRACT_STORAGE_VERSION))?;

        SingleUserAccess::new(
            crate::access_control::LEASE_CODE_ADMIN_KEY,
            self.msg.lease_code_admin.clone(),
        )
        .store(self.deps.storage)?;

        LiquidityPool::<LPN>::store(self.deps.storage, self.msg.into())?;

        Ok(Response::new().add_attribute("method", "instantiate"))
    }

    pub fn cmd(deps: DepsMut<'a>, msg: InstantiateMsg) -> ContractResult<Response> {
        let context = Self { deps, msg };

        visit_any_on_ticker::<Lpns, _>(&context.msg.lpn_ticker.clone(), context)
    }
}

impl<'a> AnyVisitor for InstantiateWithLpn<'a> {
    type Output = Response;
    type Error = ContractError;

    fn on<LPN>(self) -> AnyVisitorResult<Self>
    where
        LPN: 'static + Currency + DeserializeOwned + Serialize,
    {
        self.do_work::<LPN>()
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn instantiate(
    deps: DepsMut<'_>,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    // TODO move these checks on deserialization
    finance::currency::validate::<Lpns>(&msg.lpn_ticker)?;
    deps.api.addr_validate(msg.lease_code_admin.as_str())?;
    InstantiateWithLpn::cmd(deps, msg)
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn migrate(deps: DepsMut<'_>, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    versioning::update_software(deps.storage, version!(CONTRACT_STORAGE_VERSION))?;

    SingleUserAccess::remove_contract_owner(deps.storage);

    response::response(versioning::release()).map(Into::into)
}

struct ExecuteWithLpn<'a> {
    deps: DepsMut<'a>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
}

impl<'a> ExecuteWithLpn<'a> {
    fn do_work<LPN>(self) -> ContractResult<Response>
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
    ) -> ContractResult<Response> {
        let context = Self {
            deps,
            env,
            info,
            msg,
        };

        let config = Config::load(context.deps.storage)?;

        visit_any_on_ticker::<Lpns, _>(config.lpn_ticker(), context)
    }
}

impl<'a> AnyVisitor for ExecuteWithLpn<'a> {
    type Output = Response;
    type Error = ContractError;

    fn on<LPN>(self) -> AnyVisitorResult<Self>
    where
        LPN: 'static + Currency + DeserializeOwned + Serialize,
    {
        self.do_work::<LPN>()
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    // no currency context variants
    match msg {
        ExecuteMsg::NewLeaseCode { lease_code_id } => {
            config::try_update_lease_code(deps, info, lease_code_id)
        }
        ExecuteMsg::DistributeRewards() => rewards::try_distribute_rewards(deps, info),
        ExecuteMsg::ClaimRewards { other_recipient } => {
            rewards::try_claim_rewards(deps, env, info, other_recipient)
        }
        _ => ExecuteWithLpn::cmd(deps, env, info, msg),
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn sudo(deps: DepsMut<'_>, _env: Env, msg: SudoMsg) -> ContractResult<Response> {
    // no currency context variants
    match msg {
        SudoMsg::NewBorrowRate {
            borrow_rate: interest_rate,
        } => config::try_update_parameters(deps, interest_rate),
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
            QueryMsg::Loan { lease_addr } => {
                to_binary(&borrow::query_loan::<LPN>(self.deps.storage, lease_addr)?)
            }
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

        visit_any_on_ticker::<Lpns, _>(config.lpn_ticker(), context)
    }
}

impl<'a> AnyVisitor for QueryWithLpn<'a> {
    type Output = Binary;
    type Error = ContractError;

    fn on<LPN>(self) -> AnyVisitorResult<Self>
    where
        LPN: 'static + Currency + DeserializeOwned + Serialize,
    {
        self.do_work::<LPN>()
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn query(deps: Deps<'_>, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
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
