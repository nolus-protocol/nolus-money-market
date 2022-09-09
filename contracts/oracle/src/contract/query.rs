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
        let res = match self.msg {
            QueryMsg::PriceFor { currencies } => {
                let config = Config::load(self.deps.storage)?;
                let parameters =
                    Feeders::query_config(self.deps.storage, &config, self.env.block.time)?;

                to_binary(&PricesResponse {
                    prices: Feeds::with(config)
                        .get_prices::<OracleBase>(self.deps.storage, parameters, currencies)?
                        .values()
                        .cloned()
                        .collect(),
                })
            }
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
