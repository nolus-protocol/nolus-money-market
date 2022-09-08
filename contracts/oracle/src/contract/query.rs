use std::collections::HashSet;

use cosmwasm_std::{to_binary, Binary, Deps, Env, Storage};
use serde::{de::DeserializeOwned, Serialize};

use finance::{
    currency::{visit_any, AnyVisitor, Currency, SymbolOwned, Usdc},
    price::PriceDTO,
};
use marketprice::market_price::PriceFeedsError;

use crate::{
    error::ContractError,
    msg::{PriceResponse, PricesResponse, QueryMsg},
    state::config::Config,
};

use super::feed::Feeds;

pub struct QueryWithOracleBase<'a> {
    deps: Deps<'a>,
    env: Env,
    msg: QueryMsg,
}

impl<'a> QueryWithOracleBase<'a> {
    pub fn cmd(deps: Deps<'a>, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
        let visitor = Self { deps, env, msg };

        let config = Config::load(visitor.deps.storage)?;
        visit_any(&config.base_asset, visitor)
    }
}

impl<'a> AnyVisitor for QueryWithOracleBase<'a> {
    type Output = Binary;
    type Error = ContractError;

    fn on<OracleBase>(self) -> Result<Self::Output, Self::Error>
    where
        OracleBase: 'static + Currency + DeserializeOwned + Serialize,
    {
        let res = match self.msg {
            QueryMsg::PriceFor { currencies } => {
                Ok(to_binary(&query_market_price_for::<OracleBase>(
                    self.deps.storage,
                    self.env,
                    HashSet::from_iter(currencies.iter().cloned()),
                )?)?)
            }
            QueryMsg::Price { currency } => to_binary(&PriceResponse {
                price: PriceDTO::try_from(Feeds::get_price::<OracleBase>(
                    self.deps.storage,
                    self.env.block.time,
                    currency,
                )?)?,
            }),
            _ => {
                unreachable!()
            }
        }?;
        Ok(res)
    }
    fn on_unknown(self) -> Result<Self::Output, Self::Error> {
        Err(ContractError::UnknownCurrency {})
    }
}

pub fn query_market_price_for<OracleBase>(
    storage: &dyn Storage,
    env: Env,
    currencies: HashSet<SymbolOwned>,
) -> Result<PricesResponse, PriceFeedsError>
where
    OracleBase: Currency,
{
    let config = Config::load(storage)?;
    Ok(PricesResponse {
        prices: Feeds::new(config)
            .get_prices::<OracleBase>(storage, env.block.time, currencies)?
            .values()
            .cloned()
            .collect(),
    })
}

// fn query_market_price_for_single<OracleBase>(
//     storage: &dyn Storage,
//     env: Env,
// ) -> Result<PriceResponse, ContractError>
// where
//     OracleBase: 'static + Currency + Serialize,
// {
//     Ok(PriceResponse {
//         price: PriceDTO::try_from(Feeds::get_price::<OracleBase>(storage, env.block.time)?)?,
//     })
// }
