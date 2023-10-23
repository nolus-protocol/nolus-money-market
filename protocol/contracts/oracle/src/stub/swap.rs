use currency::SymbolOwned;
use oracle_platform::OracleRef;
use sdk::cosmwasm_std::QuerierWrapper;
use swap::SwapTarget;

use crate::{error::Result, msg::QueryMsg, ContractError};

pub trait SwapPath {
    fn swap_path(
        &self,
        from: SymbolOwned,
        to: SymbolOwned,
        querier: &QuerierWrapper<'_>,
    ) -> Result<Vec<SwapTarget>>;
}

impl SwapPath for OracleRef {
    fn swap_path(
        &self,
        from: SymbolOwned,
        to: SymbolOwned,
        querier: &QuerierWrapper<'_>,
    ) -> Result<Vec<SwapTarget>> {
        {
            let msg = QueryMsg::SwapPath { from, to };

            querier
                .query_wasm_smart(self.addr().clone(), &msg)
                .map_err(ContractError::StubSwapPathQuery)
        }
    }
}
