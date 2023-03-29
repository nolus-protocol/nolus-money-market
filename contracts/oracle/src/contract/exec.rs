use serde::de::DeserializeOwned;

use currency::lpn::Lpns;
use finance::currency::{visit_any_on_ticker, AnyVisitor, AnyVisitorResult, Currency};
use marketprice::SpotPrice;
use platform::contract;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{Addr, DepsMut, Env, Storage, Timestamp},
};

use crate::{error::ContractError, msg::ExecuteMsg, state::config::Config};

use super::{
    alarms::MarketAlarms,
    oracle::{feed::Feeds, feeder::Feeders, Oracle},
};

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

    fn on<OracleBase>(self) -> AnyVisitorResult<Self>
    where
        OracleBase: Currency + DeserializeOwned,
    {
        match self.msg {
            ExecuteMsg::FeedPrices { prices } => {
                if !Feeders::is_feeder(self.deps.storage, &self.sender)? {
                    return Err(ContractError::UnknownFeeder {});
                }

                try_feed_prices::<OracleBase>(
                    self.deps.storage,
                    self.env.block.time,
                    self.sender,
                    prices,
                )
            }
            ExecuteMsg::DispatchAlarms { max_count } => Oracle::<OracleBase>::load(
                self.deps.storage,
            )?
            .try_notify_alarms(self.deps.storage, self.env.block.time, max_count),
            ExecuteMsg::AddPriceAlarm { alarm } => {
                contract::validate_addr(&self.deps.querier, &self.sender)?;
                MarketAlarms::try_add_price_alarm::<OracleBase>(
                    self.deps.storage,
                    self.sender,
                    alarm,
                )?;
                Ok(Response::default())
            }
            ExecuteMsg::RemovePriceAlarm {} => {
                MarketAlarms::remove(self.deps.storage, self.sender)?;

                Ok(Response::default())
            }
        }
    }
}

fn try_feed_prices<OracleBase>(
    storage: &mut dyn Storage,
    block_time: Timestamp,
    sender: Addr,
    prices: Vec<SpotPrice>,
) -> Result<Response, ContractError>
where
    OracleBase: Currency + DeserializeOwned,
{
    let config = Config::load(storage)?;
    let oracle = Feeds::<OracleBase>::with(config.price_config);

    if !prices.is_empty() {
        oracle.feed_prices(storage, block_time, &sender, &prices)?;
    }

    Ok(Response::default())
}
