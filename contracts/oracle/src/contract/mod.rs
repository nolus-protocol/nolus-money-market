#[cfg(feature = "cosmwasm-bindings")]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response,
};
use cw2::set_contract_version;
use finance::{
    currency::{visit_any, AnyVisitor, Currency},
    duration::Duration,
};
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    contract_validation::validate_contract_addr,
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    state::{supported_pairs::SupportedPairs, Config},
};

use self::{
    alarms::MarketAlarms,
    config::{query_config, try_configure},
    exec::ExecWithOracleBase,
    feeder::Feeders,
    query::QueryWithOracleBase,
};

mod alarms;
mod config;
pub mod exec;
mod feed;
mod feeder;
pub mod query;

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

struct InstantiateWithCurrency<'a> {
    deps: DepsMut<'a>,
    msg: InstantiateMsg,
    owner: Addr,
}

impl<'a> InstantiateWithCurrency<'a> {
    pub fn cmd(
        deps: DepsMut<'a>,
        msg: InstantiateMsg,
        owner: Addr,
    ) -> Result<Response, ContractError> {
        let context = Self { deps, msg, owner };
        visit_any(&context.msg.base_asset.clone(), context)
    }
}

impl<'a> AnyVisitor for InstantiateWithCurrency<'a> {
    type Output = Response;
    type Error = ContractError;

    fn on<C>(self) -> Result<Self::Output, Self::Error>
    where
        C: 'static + Currency + DeserializeOwned + Serialize,
    {
        set_contract_version(self.deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

        Config::new(
            C::SYMBOL.to_string(),
            self.owner,
            Duration::from_secs(self.msg.price_feed_period_secs),
            self.msg.feeders_percentage_needed,
            self.deps.api.addr_validate(&self.msg.timealarms_addr)?,
        )
        .store(self.deps.storage)?;

        SupportedPairs::<C>::new(self.msg.currency_paths)?.save(self.deps.storage)?;

        Ok(Response::new().add_attribute("method", "instantiate"))
    }
    fn on_unknown(self) -> Result<Self::Output, Self::Error> {
        Err(ContractError::UnknownCurrency {})
    }
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    InstantiateWithCurrency::cmd(deps, msg, info.sender)?;

    Ok(Response::default())
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::Config {} => Ok(to_binary(&query_config(deps)?)?),
        QueryMsg::Feeders {} => Ok(to_binary(&Feeders::get(deps.storage)?)?),
        QueryMsg::IsFeeder { address } => {
            Ok(to_binary(&Feeders::is_feeder(deps.storage, &address)?)?)
        }
        _ => Ok(QueryWithOracleBase::cmd(deps, env, msg)?),
    }
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Config {
            price_feed_period_secs,
            feeders_percentage_needed,
        } => try_configure(
            deps,
            info,
            Duration::from_secs(price_feed_period_secs),
            feeders_percentage_needed,
        ),
        ExecuteMsg::RegisterFeeder { feeder_address } => {
            Feeders::try_register(deps, info, feeder_address)
        }
        ExecuteMsg::RemoveFeeder { feeder_address } => {
            Feeders::try_remove(deps, info, feeder_address)
        }
        ExecuteMsg::AddPriceAlarm { alarm } => {
            validate_contract_addr(&deps.querier, &info.sender)?;
            MarketAlarms::try_add_price_alarm(deps.storage, info.sender, alarm)
        }
        ExecuteMsg::RemovePriceAlarm {} => MarketAlarms::remove(deps.storage, info.sender),
        _ => Ok(ExecWithOracleBase::cmd(deps, env, msg, info.sender)?),
    }
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let resp = match msg.result {
        cosmwasm_std::SubMsgResult::Ok(resp) => {
            let data = match resp.data {
                Some(d) => d,
                None => return Ok(err_as_ok("No data")),
            };
            MarketAlarms::remove(deps.storage, from_binary(&data)?)?;
            Response::new().add_attribute("alarm", "success")
        }
        cosmwasm_std::SubMsgResult::Err(err) => Response::new()
            .add_attribute("alarm", "error")
            .add_attribute("error", err),
    };
    Ok(resp)
}

fn err_as_ok(err: &str) -> Response {
    Response::new()
        .add_attribute("alarm", "error")
        .add_attribute("error", err)
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{from_binary, testing::mock_env};
    use finance::{
        currency::{Currency, Nls, Usdc},
        duration::Duration,
        percent::Percent,
    };

    use crate::{
        contract::query,
        msg::{ConfigResponse, QueryMsg},
        state::supported_pairs::CurrencyPair,
        tests::{dummy_instantiate_msg, setup_test, CREATOR},
    };

    #[test]
    fn proper_initialization() {
        let msg = dummy_instantiate_msg(
            Usdc::SYMBOL.to_string(),
            60,
            Percent::from_percent(50),
            vec![vec![Nls::SYMBOL.to_string(), Usdc::SYMBOL.to_string()]],
            "timealarms".to_string(),
        );
        let (deps, _) = setup_test(msg);

        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!(CREATOR.to_string(), value.owner.to_string());
        assert_eq!(Usdc::SYMBOL.to_string(), value.base_asset);
        assert_eq!(Duration::from_secs(60), value.price_feed_period);
        assert_eq!(Percent::from_percent(50), value.feeders_percentage_needed);

        let res = query(deps.as_ref(), mock_env(), QueryMsg::SupportedDenomPairs {}).unwrap();
        let value: Vec<CurrencyPair> = from_binary(&res).unwrap();
        assert_eq!(
            vec![(Nls::SYMBOL.to_string(), Usdc::SYMBOL.to_string())],
            value
        );
    }
}
