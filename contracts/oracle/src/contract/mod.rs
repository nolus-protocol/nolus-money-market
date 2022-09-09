#[cfg(feature = "cosmwasm-bindings")]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response,
};
use cw2::set_contract_version;
use finance::{
    currency::{visit_any, AnyVisitor, Currency, SymbolOwned},
    price::PriceDTO,
};
use marketprice::market_price::Parameters;
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    contract_validation::validate_contract_addr,
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, PriceResponse, QueryMsg},
    state::config::Config,
};

use self::{
    alarms::MarketAlarms,
    config::{query_config, try_configure, try_configure_supported_pairs},
    exec::ExecWithOracleBase,
    feed::PriceForCurrency,
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

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Config::new(
        msg.base_asset,
        info.sender,
        msg.price_feed_period_secs,
        msg.feeders_percentage_needed,
        msg.supported_denom_pairs,
        deps.api.addr_validate(&msg.timealarms_addr)?,
    )
    .store(deps.storage)?;

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
        QueryMsg::SupportedDenomPairs {} => Ok(to_binary(
            &Config::load(deps.storage)?.supported_denom_pairs,
        )?),
        QueryMsg::Price { currency } => {
            let config = Config::load(deps.storage)?;
            let parameters = Feeders::query_config(deps.storage, &config, env.block.time)?;

            Ok(to_binary(&PriceResponse {
                price: WithQuote::cmd(deps, currency, config.base_asset, parameters)?,
            })?)
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
            price_feed_period_secs,
            feeders_percentage_needed,
        ),
        ExecuteMsg::RegisterFeeder { feeder_address } => {
            Feeders::try_register(deps, info, feeder_address)
        }
        ExecuteMsg::SupportedDenomPairs { pairs } => {
            try_configure_supported_pairs(deps.storage, info, pairs)
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

// -----------------------  trait definition ---------------------------
pub struct WithQuote<'a> {
    deps: Deps<'a>,
    base: SymbolOwned,
    quote: SymbolOwned,
    parameters: Parameters,
}

impl<'a> WithQuote<'a> {
    pub fn cmd(
        deps: Deps<'a>,
        base: SymbolOwned,
        quote: SymbolOwned,
        parameters: Parameters,
    ) -> Result<PriceDTO, ContractError> {
        let visitor = Self {
            deps,
            base,
            quote,
            parameters,
        };

        visit_any(&visitor.quote.clone(), visitor)
    }
}

impl<'a> AnyVisitor for WithQuote<'a> {
    type Output = PriceDTO;
    type Error = ContractError;

    fn on<QuoteC>(self) -> Result<Self::Output, Self::Error>
    where
        QuoteC: 'static + Currency + DeserializeOwned + Serialize,
    {
        Ok(PriceForCurrency::<QuoteC>::cmd(
            self.deps.storage,
            self.base,
            self.parameters,
        )?)
    }
    fn on_unknown(self) -> Result<Self::Output, Self::Error> {
        Err(ContractError::UnknownCurrency {})
    }
}

// -----------------------  trait definition ---------------------------

#[cfg(test)]
mod tests {
    use cosmwasm_std::{from_binary, testing::mock_env};

    use crate::{
        contract::query,
        msg::{ConfigResponse, QueryMsg},
        tests::{dummy_instantiate_msg, setup_test, CREATOR},
    };

    #[test]
    fn proper_initialization() {
        let msg = dummy_instantiate_msg(
            "token".to_string(),
            60,
            50,
            vec![("unolus".to_string(), "uosmo".to_string())],
            "timealarms".to_string(),
        );
        let (deps, _) = setup_test(msg);

        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!(CREATOR.to_string(), value.owner.to_string());
        assert_eq!("token".to_string(), value.base_asset);
        assert_eq!(60, value.price_feed_period_secs);
        assert_eq!(50, value.feeders_percentage_needed);

        let res = query(deps.as_ref(), mock_env(), QueryMsg::SupportedDenomPairs {}).unwrap();
        let value: Vec<(String, String)> = from_binary(&res).unwrap();
        assert_eq!("unolus".to_string(), value.get(0).unwrap().0);
        assert_eq!("uosmo".to_string(), value.get(0).unwrap().1);
    }
}
