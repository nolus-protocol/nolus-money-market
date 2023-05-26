use serde::{de::DeserializeOwned, Serialize};

use currency::lpn::Lpns;
use finance::currency::{visit_any_on_ticker, AnyVisitor, AnyVisitorResult, Currency};
use sdk::cosmwasm_std::{to_binary, Binary, Deps, Env};

use crate::{
    contract::oracle::Oracle,
    error::ContractError,
    msg::{PricesResponse, QueryMsg, SupportedCurrencyPairsResponse, SwapTreeResponse},
    result::ContractResult,
    state::{config::Config, supported_pairs::SupportedPairs},
};

pub(crate) struct QueryWithOracleBase<'a> {
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
        match self.msg {
            QueryMsg::SupportedCurrencyPairs {} => {
                /* Specified type to enforce proper result */
                to_binary::<SupportedCurrencyPairsResponse>(
                    &SupportedPairs::<OracleBase>::load(self.deps.storage)?
                        .supported_pairs_with_dex_symbol()
                        .collect::<ContractResult<Vec<_>>>()?,
                )
            }
            QueryMsg::Price { currency } => to_binary(
                &Oracle::<'_, _, OracleBase>::load(self.deps.storage)?
                    .try_query_price(self.env.block.time, &currency)?,
            ),
            QueryMsg::Prices {} => {
                let prices = Oracle::<'_, _, OracleBase>::load(self.deps.storage)?
                    .try_query_prices(self.env.block.time)?;

                to_binary(&PricesResponse { prices })
            }
            QueryMsg::SwapPath { from, to } => to_binary(
                &SupportedPairs::<OracleBase>::load(self.deps.storage)?
                    .load_swap_path(&from, &to)?,
            ),
            QueryMsg::SwapTree {} => to_binary(&SwapTreeResponse {
                tree: SupportedPairs::<OracleBase>::load(self.deps.storage)?
                    .query_swap_tree()
                    .into_human_readable(),
            }),
            QueryMsg::AlarmsStatus {} => to_binary(
                &Oracle::<'_, _, OracleBase>::load(self.deps.storage)?
                    .try_query_alarms(self.env.block.time)?,
            ),
            _ => {
                unreachable!() // should be done already
            }
        }
        .map_err(Into::into)
    }
}
