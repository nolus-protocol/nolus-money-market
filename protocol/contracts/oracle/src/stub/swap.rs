use currency::SymbolOwned;
use sdk::cosmwasm_std::QuerierWrapper;

use crate::api::swap::{Error, QueryMsg, Result, SwapTarget};

use super::OracleRef;

pub trait SwapPath {
    fn swap_path(
        &self,
        from: SymbolOwned,
        to: SymbolOwned,
        querier: QuerierWrapper<'_>,
    ) -> Result<Vec<SwapTarget>>;
}

impl<OracleBase> SwapPath for OracleRef<OracleBase> {
    fn swap_path(
        &self,
        from: SymbolOwned,
        to: SymbolOwned,
        querier: QuerierWrapper<'_>,
    ) -> Result<Vec<SwapTarget>> {
        {
            querier
                .query_wasm_smart(self.addr().clone(), &QueryMsg::SwapPath { from, to })
                .map_err(Error::StubSwapPathQuery)
        }
    }
}
