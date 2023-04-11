use serde::de::DeserializeOwned;

use currency::lpn::Lpns;
use finance::currency::{visit_any_on_ticker, AnyVisitor, AnyVisitorResult, Currency};
use marketprice::SpotPrice;
use platform::{contract, message::Response as MessageResponse};
use sdk::cosmwasm_std::{Addr, DepsMut, Env, Storage, Timestamp};

use crate::{error::ContractError, msg::ExecuteMsg, result::ContractResult, state::config::Config};

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
    ) -> ContractResult<MessageResponse> {
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
    type Output = MessageResponse;
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
                )
                .map(|()| Default::default())
            }
            ExecuteMsg::RemovePriceAlarm {} => {
                MarketAlarms::remove(self.deps.storage, self.sender).map(|()| Default::default())
            }
        }
    }
}

fn try_feed_prices<OracleBase>(
    storage: &mut dyn Storage,
    block_time: Timestamp,
    sender: Addr,
    prices: Vec<SpotPrice>,
) -> ContractResult<MessageResponse>
where
    OracleBase: Currency + DeserializeOwned,
{
    let config = Config::load(storage)?;
    let oracle = Feeds::<OracleBase>::with(config.price_config);

    oracle
        .feed_prices(storage, block_time, &sender, &prices)
        .map(|()| Default::default())
}
