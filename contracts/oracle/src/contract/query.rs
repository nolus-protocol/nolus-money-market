use serde::{de::DeserializeOwned, Serialize};

use currency::lpn::Lpns;
use finance::currency::{visit_any_on_ticker, AnyVisitor, Currency};
use marketprice::error::PriceFeedsError;
use sdk::cosmwasm_std::{to_binary, Binary, Deps, Env};

use crate::{
    msg::{PricesResponse, QueryMsg, SwapTreeResponse},
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
        visit_any_on_ticker::<Lpns, _>(&config.base_asset, visitor)
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
            QueryMsg::SupportedCurrencyPairs {} => Ok(to_binary(
                &SupportedPairs::<OracleBase>::load(self.deps.storage)?.query_supported_pairs(),
            )?),

            QueryMsg::Price { currency } => {
                let config = Config::load(self.deps.storage)?;
                let parameters =
                    Feeders::query_config(self.deps.storage, &config, self.env.block.time)?;
                if let Some(price) = Feeds::<OracleBase>::with(config)
                    .get_prices(self.deps.storage, parameters, &[currency])?
                    .last()
                {
                    Ok(to_binary(price)?)
                } else {
                    Err(ContractError::PriceFeedsError(PriceFeedsError::NoPrice()))
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
                        &currencies,
                    )?,
                })?)
            }
            QueryMsg::SwapPath { from, to } => Ok(to_binary(
                &SupportedPairs::<OracleBase>::load(self.deps.storage)?
                    .load_swap_path(&from, &to)?,
            )?),
            QueryMsg::SwapTree {} => Ok(to_binary(&SwapTreeResponse {
                tree: SupportedPairs::<OracleBase>::load(self.deps.storage)?.query_swap_tree(),
            })?),
            _ => {
                unreachable!() // should be done already
            }
        }?;
        Ok(res)
    }
}
