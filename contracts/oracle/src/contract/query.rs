use std::collections::HashSet;

use serde::{de::DeserializeOwned, Serialize};

use currency::lpn::Lpns;
use finance::currency::{visit_any, AnyVisitor, Currency};
use marketprice::error::PriceFeedsError;
use sdk::cosmwasm_std::{to_binary, Binary, Deps, Env};

use crate::{
    msg::{PricesResponse, QueryMsg},
    state::{supported_pairs::SupportedPairs, Config},
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

impl<'a> AnyVisitor<Lpns> for QueryWithOracleBase<'a> {
    type Output = Binary;
    type Error = ContractError;

    fn on<OracleBase>(self) -> Result<Self::Output, Self::Error>
    where
        OracleBase: 'static + Currency + DeserializeOwned + Serialize,
    {
        let res = match self.msg {
            QueryMsg::SupportedDenomPairs {} => Ok(to_binary(
                &SupportedPairs::<OracleBase>::load(self.deps.storage)?.query_supported_pairs(),
            )?),

            QueryMsg::Price { currency } => {
                let config = Config::load(self.deps.storage)?;
                let parameters =
                    Feeders::query_config(self.deps.storage, &config, self.env.block.time)?;
                match Feeds::<OracleBase>::with(config)
                    .get_prices(self.deps.storage, parameters, HashSet::from([currency]))?
                    .first()
                {
                    Some(price) => Ok(to_binary(price)?),
                    None => Err(ContractError::PriceFeedsError(PriceFeedsError::NoPrice())),
                }
            }
            QueryMsg::Prices { currencies } => {
                let config = Config::load(self.deps.storage)?;
                let parameters =
                    Feeders::query_config(self.deps.storage, &config, self.env.block.time)?;
                Ok(to_binary(&PricesResponse {
                    prices: Feeds::<OracleBase>::with(config).get_prices(
                        self.deps.storage,
                        parameters,
                        currencies,
                    )?,
                })?)
            }
            _ => {
                unreachable!()
            } // should be done already
        }?;
        Ok(res)
    }
}
