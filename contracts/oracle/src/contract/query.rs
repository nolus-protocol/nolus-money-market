use std::collections::HashSet;

use cosmwasm_std::{to_binary, Binary, Deps, Env};
use finance::currency::{visit_any, AnyVisitor, Currency};
use marketprice::error::PriceFeedsError;
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    msg::{PriceResponse, PricesResponse, QueryMsg},
    state::{config::Config, supported_pairs::SupportedPairs},
    ContractError,
};

use super::{feed::Feeds, feeder::Feeders};

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
        let config = Config::load(self.deps.storage)?;
        let parameters = Feeders::query_config(self.deps.storage, &config, self.env.block.time)?;

        let res = match self.msg {
            QueryMsg::SupportedDenomPairs {} => Ok(to_binary(
                &SupportedPairs::<OracleBase>::load(self.deps.storage)?.query_supported_pairs(),
            )?),

            QueryMsg::Price { currency } => {
                match Feeds::<OracleBase>::with(config)
                    .get_prices(self.deps.storage, parameters, HashSet::from([currency]))?
                    .first()
                {
                    Some(price) => Ok(to_binary(&PriceResponse {
                        price: price.to_owned(),
                    })?),
                    None => Err(ContractError::PriceFeedsError(PriceFeedsError::NoPrice())),
                }
            }
            QueryMsg::Prices { currencies } => Ok(to_binary(&PricesResponse {
                prices: Feeds::<OracleBase>::with(config).get_prices(
                    self.deps.storage,
                    parameters,
                    currencies,
                )?,
            })?),
            _ => {
                unreachable!()
            } // should be done already
        }?;
        Ok(res)
    }
    fn on_unknown(self) -> Result<Self::Output, Self::Error> {
        Err(ContractError::UnknownCurrency {})
    }
}
