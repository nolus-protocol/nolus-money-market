use serde::de::DeserializeOwned;

use currencies::Lpns;
use currency::{AnyVisitor, AnyVisitorResult, Currency, GroupVisit, Tickers};
use sdk::cosmwasm_std::Storage;

use crate::{
    api::Config, error::ContractError, result::ContractResult,
    state::supported_pairs::SupportedPairs,
};

pub(crate) fn with_oracle_base(storage: &mut dyn Storage) -> ContractResult<()> {
    Config::load(storage)
        .map_err(ContractError::LoadConfig)
        .and_then(|Config { ref base_asset, .. }: Config| {
            Tickers.visit_any::<Lpns, _>(base_asset, MigrateWithOracleBase { storage })
        })
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
        SupportedPairs::migrate(self.storage)
    }
}
