use serde::{de::DeserializeOwned, Serialize};

use currency::{AnyVisitor, AnyVisitorResult, Currency, GroupVisit, Tickers};
use sdk::cosmwasm_std::{to_json_binary, Binary, Deps, Env};

use crate::{
    api::{Config, PriceCurrencies, PricesResponse, QueryMsg, StableCurrency, SwapTreeResponse},
    contract::oracle::Oracle,
    state::supported_pairs::SupportedPairs,
    ContractError,
};

pub struct QueryWithOracleBase<'a> {
    deps: Deps<'a>,
    env: Env,
    msg: QueryMsg,
}

impl<'a> QueryWithOracleBase<'a> {
    pub fn cmd(deps: Deps<'a>, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
        let visitor = Self { deps, env, msg };

        Config::load(visitor.deps.storage)
            .map_err(ContractError::LoadConfig)
            .and_then(|config: Config| {
                Tickers.visit_any::<StableCurrency, _>(&config.base_asset, visitor)
            })
    }
}

impl<'a> AnyVisitor for QueryWithOracleBase<'a> {
    type Output = Binary;
    type Error = ContractError;

    fn on<OracleBase>(self) -> AnyVisitorResult<Self>
    where
        OracleBase: 'static + Currency + DeserializeOwned + Serialize,
    {
        type QueryOracle<'storage, S, BaseC> =
            Oracle<'storage, S, PriceCurrencies, BaseC, StableCurrency>;

        match self.msg {
            QueryMsg::SupportedCurrencyPairs {} => to_json_binary(
                &SupportedPairs::<OracleBase>::load(self.deps.storage)?
                    .swap_pairs_df()
                    .collect::<Vec<_>>(),
            ),
            QueryMsg::Price { currency } => to_json_binary(
                &QueryOracle::<'_, _, OracleBase>::load(self.deps.storage)?
                    .try_query_price(self.env.block.time, &currency)?,
            ),
            QueryMsg::Prices {} => {
                let prices = QueryOracle::<'_, _, OracleBase>::load(self.deps.storage)?
                    .try_query_prices(self.env.block.time)?;

                to_json_binary(&PricesResponse { prices })
            }
            QueryMsg::SwapPath { from, to } => to_json_binary(
                &SupportedPairs::<OracleBase>::load(self.deps.storage)?
                    .load_swap_path(&from, &to)?,
            ),
            QueryMsg::SwapTree {} => to_json_binary(&SwapTreeResponse {
                tree: SupportedPairs::<OracleBase>::load(self.deps.storage)?
                    .query_swap_tree()
                    .into_human_readable(),
            }),
            QueryMsg::AlarmsStatus {} => to_json_binary(
                &QueryOracle::<'_, _, OracleBase>::load(self.deps.storage)?
                    .try_query_alarms(self.env.block.time)?,
            ),
            _ => {
                unreachable!() // should be done already
            }
        }
        .map_err(ContractError::ConvertToBinary)
    }
}
