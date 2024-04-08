use serde::de::DeserializeOwned;

use currencies::Lpns;
use currency::{AnyVisitor, AnyVisitorResult, Currency, GroupVisit, Tickers};
use sdk::cosmwasm_std::DepsMut;

use crate::{
    api::{BaseCurrency, SudoMsg},
    error::ContractError,
    result::ContractResult,
    state::supported_pairs::SupportedPairs,
};

pub struct SudoWithOracleBase<'a> {
    deps: DepsMut<'a>,
    msg: SudoMsg,
}

impl<'a> SudoWithOracleBase<'a> {
    pub fn cmd(deps: DepsMut<'a>, msg: SudoMsg) -> ContractResult<<Self as AnyVisitor>::Output> {
        let visitor = Self { deps, msg };

        Tickers.visit_any::<Lpns, _>(BaseCurrency::TICKER, visitor)
    }
}

impl<'a> AnyVisitor for SudoWithOracleBase<'a> {
    type Output = ();
    type Error = ContractError;

    fn on<OracleBase>(self) -> AnyVisitorResult<Self>
    where
        OracleBase: Currency + DeserializeOwned,
    {
        match self.msg {
            SudoMsg::SwapTree {
                stable_currency,
                tree,
            } => SupportedPairs::<OracleBase>::new(tree.into_tree(), stable_currency)
                .and_then(|supported_pairs| supported_pairs.save(self.deps.storage)),
            _ => unreachable!(),
        }
    }
}
