use std::collections::{HashMap, HashSet};

#[cfg(feature = "cosmwasm-bindings")]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    Storage, Timestamp,
};
use cw2::set_contract_version;
use serde::{de::DeserializeOwned, Serialize};

use finance::{
    currency::{visit_any, AnyVisitor, Currency, Nls, SymbolOwned, Usdc},
    price::PriceDTO,
};
use marketprice::{
    market_price::PriceFeedsError,
    storage::{DenomPair, Price},
};

use crate::{
    alarms::MarketAlarms,
    contract_validation::validate_contract_addr,
    error::ContractError,
    msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, PriceResponse, PricesResponse, QueryMsg},
    oracle::MarketOracle,
    state::config::Config,
};

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
            try_register_feeder(deps, info, feeder_address)
        }
        ExecuteMsg::SupportedDenomPairs { pairs } => {
            try_configure_supported_pairs(deps.storage, info, pairs)
        }
        ExecuteMsg::FeedPrices { prices } => {
            try_feed_prices(deps.storage, env.block.time, info.sender, prices)
        }
        ExecuteMsg::AddPriceAlarm { alarm } => {
            validate_contract_addr(&deps.querier, &info.sender)?;
            MarketAlarms::try_add_price_alarm(deps.storage, info.sender, alarm)
        }
        ExecuteMsg::RemovePriceAlarm {} => MarketAlarms::remove(deps.storage, info.sender),
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
            QueryMsg::Price { currency } => to_binary(&query_market_price_for_single::<LPN, Usdc>(
                self.deps.storage,
                self.env,
                currency,
            )?),
            _ => {
                unreachable!()
            } // should be done already
        }?;
        Ok(res)
    }

    pub fn cmd(deps: Deps<'a>, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
        let context = Self { deps, env, msg };

        let config = Config::load(context.deps.storage)?;
        visit_any(&config.base_asset, context)
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
    fn on_unknown(self) -> Result<Self::Output, Self::Error> {
        Err(ContractError::UnknownCurrency {})
    }
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::Config {} => Ok(to_binary(&query_config(deps)?)?),
        QueryMsg::Feeders {} => Ok(to_binary(&MarketOracle::get_feeders(deps.storage)?)?),
        QueryMsg::IsFeeder { address } => Ok(to_binary(&MarketOracle::is_feeder(
            deps.storage,
            &address,
        )?)?),
        QueryMsg::PriceFor { denoms } => Ok(to_binary(&query_market_price_for(
            deps.storage,
            env,
            HashSet::from_iter(denoms.iter().cloned()),
        )?)?),
        QueryMsg::SupportedDenomPairs {} => Ok(to_binary(
            &Config::load(deps.storage)?.supported_denom_pairs,
        )?),
        _ => Ok(QueryWithLpn::cmd(deps, env, msg)?),
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

fn query_config(deps: Deps) -> Result<ConfigResponse, ContractError> {
    let config = Config::load(deps.storage)?;
    Ok(ConfigResponse {
        base_asset: config.base_asset,
        owner: config.owner,
        price_feed_period_secs: config.price_feed_period_secs,
        feeders_percentage_needed: config.feeders_percentage_needed,
    })
}

fn query_market_price_for(
    storage: &dyn Storage,
    env: Env,
    currencies: HashSet<SymbolOwned>,
) -> Result<PricesResponse, PriceFeedsError> {
    let config = Config::load(storage)?;
    Ok(PricesResponse {
        prices: MarketOracle::new(config)
            .get_prices(storage, env.block.time, currencies)?
            .values()
            .cloned()
            .collect(),
    })
}

fn query_market_price_for_single<C, QuoteC>(
    storage: &dyn Storage,
    env: Env,
    currency: SymbolOwned,
) -> Result<PriceResponse, ContractError>
where
    C: 'static + Currency + Serialize,
    QuoteC: 'static + Currency + Serialize,
{
    Ok(PriceResponse {
        price: PriceDTO::try_from(MarketOracle::get_single_price::<C, QuoteC>(
            storage,
            env.block.time,
            currency,
        )?)?,
    })
}

fn try_configure(
    deps: DepsMut,
    info: MessageInfo,
    price_feed_period_secs: u32,
    feeders_percentage_needed: u8,
) -> Result<Response, ContractError> {
    Config::update(
        deps.storage,
        price_feed_period_secs,
        feeders_percentage_needed,
        info.sender,
    )?;

    Ok(Response::new())
}

fn try_configure_supported_pairs(
    storage: &mut dyn Storage,
    info: MessageInfo,
    pairs: Vec<DenomPair>,
) -> Result<Response, ContractError> {
    Config::update_supported_pairs(storage, pairs, info.sender)?;

    Ok(Response::new())
}

fn try_register_feeder(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let config = Config::load(deps.storage)?;
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }
    // check if address is valid
    let f_address = deps.api.addr_validate(&address)?;
    MarketOracle::register_feeder(deps, f_address)?;

    Ok(Response::new())
}

fn try_feed_prices(
    storage: &mut dyn Storage,
    block_time: Timestamp,
    sender_raw: Addr,
    prices: Vec<Price>,
) -> Result<Response, ContractError> {
    // Check feeder permission
    let is_registered = MarketOracle::is_feeder(storage, &sender_raw)?;
    if !is_registered {
        return Err(ContractError::UnknownFeeder {});
    }

    let config = Config::load(storage)?;
    let oracle = MarketOracle::new(config.clone());

    // Store the new price feed
    oracle.feed_prices(storage, block_time, &sender_raw, prices)?;

    // Get all currencies registered for alarms
    let hooks_currencies = MarketAlarms::get_hooks_currencies(storage)?;

    //re-calculate the price of these currencies
    let updated_prices: HashMap<SymbolOwned, Price> =
        oracle.get_prices(storage, block_time, hooks_currencies)?;

    // try notify affected subscribers
    let mut batch = MarketAlarms::try_notify_hooks(storage, updated_prices)?;
    batch.schedule_execute_wasm_reply_error::<_, Nls>(
        &config.timealarms_contract,
        timealarms::msg::ExecuteMsg::Notify(),
        None,
        1,
    )?;
    Ok(Response::from(batch))
}
