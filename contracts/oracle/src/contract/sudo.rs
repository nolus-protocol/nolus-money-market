use serde::de::DeserializeOwned;

use currency::lpn::Lpns;
use currency::{self, AnyVisitor, AnyVisitorResult, Currency};
use sdk::cosmwasm_std::DepsMut;

use crate::{
    error::ContractError,
    msg::SudoMsg,
    result::ContractResult,
    state::{config::Config, supported_pairs::SupportedPairs},
};

pub struct SudoWithOracleBase<'a> {
    deps: DepsMut<'a>,
    msg: SudoMsg,
}

impl<'a> SudoWithOracleBase<'a> {
    pub fn cmd(deps: DepsMut<'a>, msg: SudoMsg) -> ContractResult<<Self as AnyVisitor>::Output> {
        let visitor = Self { deps, msg };

        Config::load(visitor.deps.storage)
            .map_err(ContractError::LoadConfig)
            .and_then(|config: Config| {
                currency::visit_any_on_ticker::<Lpns, _>(&config.base_asset, visitor)
            })
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
            SudoMsg::SwapTree { tree } => SupportedPairs::<OracleBase>::new(tree.into_tree())?
                .validate_tickers()?
                .save(self.deps.storage),
            _ => unreachable!(),
        }
    }
}
