use currency::lpn::Lpns;
use finance::currency::{visit_any_on_ticker, AnyVisitor, Currency};
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{Addr, DepsMut, Env},
};
use serde::de::DeserializeOwned;

use crate::{
    error::ContractError,
    msg::ExecuteMsg,
    state::{supported_pairs::SupportedPairs, Config},
};

use super::{feed, feeder::Feeders};

pub struct ExecWithOracleBase<'a> {
    deps: DepsMut<'a>,
    env: Env,
    msg: ExecuteMsg,
    sender: Addr,
}

impl<'a> ExecWithOracleBase<'a> {
    pub fn cmd(
        deps: DepsMut<'a>,
        env: Env,
        msg: ExecuteMsg,
        sender: Addr,
    ) -> Result<Response, ContractError> {
        let visitor = Self {
            deps,
            env,
            msg,
            sender,
        };

        let config = Config::load(visitor.deps.storage)?;
        visit_any_on_ticker::<Lpns, _>(&config.base_asset, visitor)
    }
}

impl<'a> AnyVisitor for ExecWithOracleBase<'a> {
    type Output = Response;
    type Error = ContractError;

    fn on<OracleBase>(self) -> Result<Self::Output, Self::Error>
    where
        OracleBase: Currency + DeserializeOwned,
    {
        match self.msg {
            ExecuteMsg::SwapTree { tree } => {
                crate::access_control::OWNER
                    .assert_address::<_, Self::Error>(self.deps.as_ref(), &self.sender)?;

                SupportedPairs::<OracleBase>::new(tree)?
                    .validate_tickers()?
                    .save(self.deps.storage)?;
                Ok(Response::default())
            }
            ExecuteMsg::FeedPrices { prices } => {
                if !Feeders::is_feeder(self.deps.storage, &self.sender)? {
                    return Err(ContractError::UnknownFeeder {});
                }

                feed::try_feed_prices::<OracleBase>(
                    self.deps.storage,
                    self.env.block.time,
                    self.sender,
                    prices,
                )
            }
            ExecuteMsg::DispatchAlarms { max_count } => feed::try_notify_alarms::<OracleBase>(
                self.deps.storage,
                self.env.block.time,
                max_count,
            ),
            _ => {
                unreachable!()
            }
        }
    }
}
