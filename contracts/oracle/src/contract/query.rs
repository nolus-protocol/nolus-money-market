use cosmwasm_std::{Storage, Timestamp};
use serde::{de::DeserializeOwned, Serialize};

use currency::lpn::Lpns;
use finance::currency::{visit_any_on_ticker, AnyVisitor, AnyVisitorResult, Currency, SymbolOwned};
use marketprice::error::PriceFeedsError;
use marketprice::SpotPrice;
use sdk::cosmwasm_std::{to_binary, Binary, Deps, Env};

use crate::{
    msg::{PricesResponse, QueryMsg, SwapTreeResponse},
    state::{config::Config, supported_pairs::SupportedPairs},
    ContractError,
};

use super::{
    feed::{try_query_alarms, Feeds},
    feeder::Feeders,
};

pub struct QueryWithOracleBase<'a> {
    deps: Deps<'a>,
    env: Env,
    msg: QueryMsg,
}

impl<'a> QueryWithOracleBase<'a> {
    pub fn cmd(deps: Deps<'a>, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
        let visitor = Self { deps, env, msg };

        let config = Config::load(visitor.deps.storage)?;
        visit_any_on_ticker::<Lpns, _>(&config.base_asset, visitor)
    }
}

impl<'a> AnyVisitor for QueryWithOracleBase<'a> {
    type Output = Binary;
    type Error = ContractError;

    fn on<OracleBase>(self) -> AnyVisitorResult<Self>
    where
        OracleBase: 'static + Currency + DeserializeOwned + Serialize,
    {
        let res = match self.msg {
            QueryMsg::SupportedCurrencyPairs {} => Ok(to_binary(
                &SupportedPairs::<OracleBase>::load(self.deps.storage)?
                    .swap_pairs_df()
                    .collect::<Vec<_>>(),
            )?),

            QueryMsg::Price { currency } => {
                let prices =
                    calc_prices::<OracleBase>(self.deps.storage, self.env.block.time, &[currency])?;

                if let Some(price) = prices.first() {
                    Ok(to_binary(price)?)
                } else {
                    // TODO check whether this branch is reachable at all
                    Err(ContractError::PriceFeedsError(PriceFeedsError::NoPrice()))
                }
            }
            QueryMsg::Prices { currencies } => {
                let prices =
                    calc_prices::<OracleBase>(self.deps.storage, self.env.block.time, &currencies)?;
                Ok(to_binary(&PricesResponse { prices })?)
            }
            QueryMsg::SwapPath { from, to } => Ok(to_binary(
                &SupportedPairs::<OracleBase>::load(self.deps.storage)?
                    .load_swap_path(&from, &to)?,
            )?),
            QueryMsg::SwapTree {} => Ok(to_binary(&SwapTreeResponse {
                tree: SupportedPairs::<OracleBase>::load(self.deps.storage)?
                    .query_swap_tree()
                    .into_human_readable(),
            })?),
            QueryMsg::AlarmsStatus {} => Ok(to_binary(&try_query_alarms::<OracleBase>(
                self.deps.storage,
                self.env.block.time,
            )?)?),
            _ => {
                unreachable!() // should be done already
            }
        }?;
        Ok(res)
    }
}

fn calc_prices<OracleBase>(
    storage: &dyn Storage,
    at: Timestamp,
    currencies: &[SymbolOwned],
) -> Result<Vec<SpotPrice>, ContractError>
where
    OracleBase: 'static + Currency + DeserializeOwned + Serialize,
{
    let total_feeders = Feeders::total_registered(storage)?;
    let config = Config::load(storage)?;
    let feeds = Feeds::<OracleBase>::with(config.price_config);
    let prices = feeds.calc_prices(storage, at, total_feeders, currencies)?;
    Ok(prices)
}
