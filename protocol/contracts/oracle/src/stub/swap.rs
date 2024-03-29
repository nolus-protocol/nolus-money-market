use currency::SymbolOwned;
use oracle_platform::OracleRef;
use sdk::cosmwasm_std::QuerierWrapper;

use crate::api::swap::{Error, QueryMsg, Result, SwapTarget};

pub trait SwapPath {
    fn swap_path(
        &self,
        from: SymbolOwned,
        to: SymbolOwned,
        querier: QuerierWrapper<'_>,
    ) -> Result<Vec<SwapTarget>>;
}

impl SwapPath for OracleRef {
    fn swap_path(
        &self,
        from: SymbolOwned,
        to: SymbolOwned,
        querier: QuerierWrapper<'_>,
    ) -> Result<Vec<SwapTarget>> {
        {
            let msg = QueryMsg::SwapPath { from, to };

            querier
                .query_wasm_smart(self.addr().clone(), &msg)
                .map_err(Error::StubSwapPathQuery)
        }
    }
}
