use currency::{Currency, Group, SymbolOwned};
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

impl<OracleBase, OracleBaseG> SwapPath for OracleRef<OracleBase, OracleBaseG>
where
    OracleBase: Currency,
    OracleBaseG: Group,
{
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
