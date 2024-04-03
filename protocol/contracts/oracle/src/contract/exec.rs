use serde::de::DeserializeOwned;

use currencies::Lpns;
use currency::{AnyVisitor, AnyVisitorResult, Currency, Group, GroupVisit, Tickers};
use finance::price::dto::PriceDTO;
use platform::{contract, response};
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{Addr, DepsMut, Env, Storage, Timestamp},
};

use crate::{
    api::{Config, DispatchAlarmsResponse, ExecuteMsg, PriceCurrencies},
    contract::{alarms::MarketAlarms, oracle::Oracle},
    error::ContractError,
    result::ContractResult,
};

use super::oracle::{feed::Feeds, feeder::Feeders};

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
    ) -> ContractResult<CwResponse> {
        let visitor = Self {
            deps,
            env,
            msg,
            sender,
        };

        Config::load(visitor.deps.storage)
            .map_err(ContractError::LoadConfig)
            .and_then(|config: Config| Tickers.visit_any::<Lpns, _>(&config.base_asset, visitor))
    }
}

impl<'a> AnyVisitor for ExecWithOracleBase<'a> {
    type Output = CwResponse;
    type Error = ContractError;

    fn on<BaseC>(self) -> AnyVisitorResult<Self>
    where
        BaseC: Currency + DeserializeOwned,
    {
        match self.msg {
            ExecuteMsg::FeedPrices { prices } => {
                if !Feeders::is_feeder(self.deps.storage, &self.sender)
                    .map_err(ContractError::LoadFeeders)?
                {
                    return Err(ContractError::UnknownFeeder {});
                }

                try_feed_prices::<PriceCurrencies>(
                    self.deps.storage,
                    self.env.block.time,
                    self.sender,
                    prices,
                )
                .map(|()| Default::default())
            }
            ExecuteMsg::DispatchAlarms { max_count } => {
                Oracle::<'_, _, PriceCurrencies>::load(self.deps.storage)?
                    .try_notify_alarms(self.env.block.time, max_count)
                    .and_then(|(total, resp)| {
                        response::response_with_messages(DispatchAlarmsResponse(total), resp)
                    })
            }
            ExecuteMsg::AddPriceAlarm { alarm } => {
                contract::validate_addr(self.deps.querier, &self.sender)?;

                MarketAlarms::new(self.deps.storage)
                    .try_add_price_alarm(self.sender, alarm)
                    .map(|()| Default::default())
            }
        }
    }
}

fn try_feed_prices<G>(
    storage: &mut dyn Storage,
    block_time: Timestamp,
    sender: Addr,
    prices: Vec<PriceDTO<G, G>>,
) -> ContractResult<()>
where
    G: Group,
{
    let config = Config::load(storage).map_err(ContractError::LoadConfig)?;
    let oracle = Feeds::with(config.price_config);

    oracle
        .feed_prices(storage, block_time, &sender, &prices)
        .map(|()| Default::default())
}
