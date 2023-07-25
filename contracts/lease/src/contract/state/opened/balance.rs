use currency::{lpn::Lpns, AnyVisitor, AnyVisitorResult, Currency, Symbol};
use platform::bank;
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::{
    api::LpnCoin,
    error::{ContractError, ContractResult},
};

pub(super) fn balance(
    account: &Addr,
    currency: Symbol<'_>,
    querier: &QuerierWrapper<'_>,
) -> ContractResult<LpnCoin> {
    currency::visit_any_on_ticker::<Lpns, _>(currency, CheckBalance { account, querier })
}

struct CheckBalance<'a> {
    account: &'a Addr,
    querier: &'a QuerierWrapper<'a>,
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
