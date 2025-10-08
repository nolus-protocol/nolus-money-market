use currency::{Currency, CurrencyDTO, Group, MemberOf};
use oracle_platform::OracleRef;
use sdk::cosmwasm_std::QuerierWrapper;

use crate::api::swap::{Error, QueryMsg, Result, SwapTarget};

pub trait SwapPath<SwapGroup>
where
    SwapGroup: Group,
{
    fn swap_path<SwapIn, SwapOut>(
        &self,
        from: CurrencyDTO<SwapIn>,
        to: CurrencyDTO<SwapOut>,
        querier: QuerierWrapper<'_>,
    ) -> Result<Vec<SwapTarget<SwapGroup>>>
    where
        SwapIn: Group + MemberOf<SwapGroup>,
        SwapOut: Group + MemberOf<SwapGroup>;
}

impl<SwapGroup, OracleBase, OracleBaseG> SwapPath<SwapGroup> for OracleRef<OracleBase, OracleBaseG>
where
    SwapGroup: Group,
    OracleBase: Currency + MemberOf<OracleBaseG>,
    OracleBaseG: Group,
{
    fn swap_path<SwapIn, SwapOut>(
        &self,
        from: CurrencyDTO<SwapIn>,
        to: CurrencyDTO<SwapOut>,
        querier: QuerierWrapper<'_>,
    ) -> Result<Vec<SwapTarget<SwapGroup>>>
    where
        SwapIn: Group + MemberOf<SwapGroup>,
        SwapOut: Group + MemberOf<SwapGroup>,
    {
        querier
            .query_wasm_smart(
                self.addr().clone(),
                &QueryMsg::SwapPath {
                    from: from.into_super_group::<SwapGroup>(),
                    to: to.into_super_group::<SwapGroup>(),
                },
            )
            .map_err(Error::stub_swap_path_query)
    }
}
