use currency::{AnyVisitor, AnyVisitorResult, Currency, GroupVisit, SymbolSlice, Tickers};
use platform::bank;
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::{
    error::{ContractError, ContractResult},
    finance::{LpnCoinDTO, LpnCurrencies},
};

pub(super) fn balance(
    account: &Addr,
    currency: &SymbolSlice,
    querier: QuerierWrapper<'_>,
) -> ContractResult<LpnCoinDTO> {
    Tickers.visit_any::<LpnCurrencies, _>(currency, CheckBalance { account, querier })
}

struct CheckBalance<'a> {
    account: &'a Addr,
    querier: QuerierWrapper<'a>,
}
impl<'a> AnyVisitor for CheckBalance<'a> {
    type Output = LpnCoinDTO;
    type Error = ContractError;

    fn on<C>(self) -> AnyVisitorResult<Self>
    where
        C: Currency,
    {
        bank::balance::<C>(self.account, self.querier)
            .map(Into::into)
            .map_err(Into::into)
    }
}
