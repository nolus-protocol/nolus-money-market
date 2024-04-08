use serde::de::DeserializeOwned;

use currencies::Lpns;
use currency::{AnyVisitor, AnyVisitorResult, Currency, GroupVisit, Tickers};
use sdk::cosmwasm_std::Storage;

use crate::{
    api::BaseCurrency, error::ContractError, result::ContractResult,
    state::supported_pairs::SupportedPairs,
};

pub(crate) fn with_oracle_base(storage: &mut dyn Storage) -> ContractResult<()> {
    Tickers.visit_any::<Lpns, _>(BaseCurrency::TICKER, MigrateWithOracleBase { storage })
}

struct MigrateWithOracleBase<'a> {
    storage: &'a mut dyn Storage,
}

impl<'a> AnyVisitor for MigrateWithOracleBase<'a> {
    type Output = ();
    type Error = ContractError;

    fn on<OracleBase>(self) -> AnyVisitorResult<Self>
    where
        OracleBase: Currency + DeserializeOwned,
    {
        SupportedPairs::<OracleBase>::migrate(self.storage)
    }
}
