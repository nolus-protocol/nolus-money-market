use cosmwasm_std::{to_binary, Binary, Deps, Env};
use serde::{de::DeserializeOwned, Serialize};

use finance::{
    currency::{visit_any, AnyVisitor, Currency},
    price::PriceDTO,
};

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
                let config = Config::load(self.deps.storage)?;
                to_binary(&PricesResponse {
                    prices: Feeds::with(config)
                        .get_prices::<OracleBase>(
                            self.deps.storage,
                            self.env.block.time,
                            currencies,
                        )?
                        .values()
                        .cloned()
                        .collect(),
                })
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
