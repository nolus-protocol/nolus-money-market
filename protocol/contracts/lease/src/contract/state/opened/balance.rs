use currencies::Lpns;
use currency::{AnyVisitor, AnyVisitorResult, Currency, GroupVisit, SymbolSlice, Tickers};
use platform::bank;
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::{
    api::LpnCoin,
    error::{ContractError, ContractResult},
};

pub(super) fn balance(
    account: &Addr,
    currency: &SymbolSlice,
    querier: QuerierWrapper<'_>,
) -> ContractResult<LpnCoin> {
    Tickers.visit_any::<Lpns, _>(currency, CheckBalance { account, querier })
}

struct CheckBalance<'a> {
    account: &'a Addr,
    querier: QuerierWrapper<'a>,
}
impl<'a> AnyVisitor for CheckBalance<'a> {
    type Output = LpnCoin;
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
