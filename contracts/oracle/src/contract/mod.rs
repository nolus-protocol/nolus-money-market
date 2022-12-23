use access_control::SingleUserAccess;
use currency::lpn::Lpns;
use finance::currency::{visit_any_on_ticker, AnyVisitor, Currency};
use platform::contract;
#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{from_binary, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply},
    cw2::set_contract_version,
};

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    state::supported_pairs::SupportedPairs,
};

use self::{
    alarms::MarketAlarms,
    config::{query_config, try_configure},
    exec::ExecWithOracleBase,
    feeder::Feeders,
    query::QueryWithOracleBase,
};

mod alarms;
mod config;
pub mod exec;
mod feed;
mod feeder;
pub mod query;

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

struct InstantiateWithCurrency<'a> {
    deps: DepsMut<'a>,
    msg: InstantiateMsg,
    owner: Addr,
}

impl<'a> InstantiateWithCurrency<'a> {
    pub fn cmd(
        deps: DepsMut<'a>,
        msg: InstantiateMsg,
        owner: Addr,
    ) -> Result<Response, ContractError> {
        let context = Self { deps, msg, owner };
        visit_any_on_ticker::<Lpns, _>(&context.msg.config.base_asset.clone(), context)
    }
}

impl<'a> AnyVisitor for InstantiateWithCurrency<'a> {
    type Output = Response;
    type Error = ContractError;

    fn on<C>(self) -> Result<Self::Output, Self::Error>
    where
        C: Currency,
    {
        SingleUserAccess::new_contract_owner(self.owner).store(self.deps.storage)?;

        self.msg.config.store(self.deps.storage)?;

        SupportedPairs::<C>::new(self.msg.swap_tree)?
            .validate_tickers()?
            .save(self.deps.storage)?;

        Ok(Response::new().add_attribute("method", "instantiate"))
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    InstantiateWithCurrency::cmd(deps, msg, info.sender)?;

    Ok(Response::default())
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::Config {} => Ok(to_binary(&query_config(deps.storage)?)?),
        QueryMsg::Feeders {} => Ok(to_binary(&Feeders::get(deps.storage)?)?),
        QueryMsg::IsFeeder { address } => {
            Ok(to_binary(&Feeders::is_feeder(deps.storage, &address)?)?)
        }
        _ => Ok(QueryWithOracleBase::cmd(deps, env, msg)?),
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig(price_config) => try_configure(deps.storage, info, price_config),
        ExecuteMsg::RegisterFeeder { feeder_address } => {
            Feeders::try_register(deps, info, feeder_address)
        }
        ExecuteMsg::RemoveFeeder { feeder_address } => {
            Feeders::try_remove(deps, info, feeder_address)
        }
        ExecuteMsg::AddPriceAlarm { alarm } => {
            contract::validate_addr(&deps.querier, &info.sender)?;

            MarketAlarms::try_add_price_alarm(deps.storage, info.sender, alarm)
        }
        ExecuteMsg::RemovePriceAlarm {} => MarketAlarms::remove(deps.storage, info.sender),
        _ => Ok(ExecWithOracleBase::cmd(deps, env, msg, info.sender)?),
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let resp = match msg.result {
        cosmwasm_std::SubMsgResult::Ok(resp) => {
            let data = match resp.data {
                Some(d) => d,
                None => return Ok(err_as_ok("No data")),
            };
            MarketAlarms::remove(deps.storage, from_binary(&data)?)?;
            Response::new().add_attribute("alarm", "success")
        }
        cosmwasm_std::SubMsgResult::Err(err) => Response::new()
            .add_attribute("alarm", "error")
            .add_attribute("error", err),
    };
    Ok(resp)
}

fn err_as_ok(err: &str) -> Response {
    Response::new()
        .add_attribute("alarm", "error")
        .add_attribute("error", err)
}

#[cfg(test)]
mod tests {
    use swap::SwapTarget;
    use trees::tr;

    use currency::{lease::Osmo, lpn::Usdc};
    use finance::{currency::Currency, duration::Duration, percent::Percent};
    use sdk::cosmwasm_std::{from_binary, testing::mock_env};

    use crate::{
        contract::query,
        msg::{ConfigResponse, QueryMsg},
        state::{
            config::Config,
            supported_pairs::{SwapLeg, TreeStore},
        },
        tests::{dummy_instantiate_msg, setup_test},
    };

    #[test]
    fn proper_initialization() {
        use marketprice::config::Config as PriceConfig;
        let msg = dummy_instantiate_msg(
            Usdc::TICKER.to_string(),
            60,
            Percent::from_percent(50),
            TreeStore(tr((0, Usdc::TICKER.to_string())) / tr((1, Osmo::TICKER.to_string()))),
        );
        let (deps, info) = setup_test(msg);

        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!(
            ConfigResponse {
                owner: info.sender,
                config: Config {
                    base_asset: Usdc::TICKER.into(),
                    price_config: PriceConfig::new(
                        Duration::from_secs(60),
                        Percent::from_percent(50),
                        Duration::from_secs(60),
                        Percent::from_percent(88),
                    )
                }
            },
            value
        );

        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::SupportedCurrencyPairs {},
        )
        .unwrap();
        let value: Vec<SwapLeg> = from_binary(&res).unwrap();

        let expected = vec![SwapLeg {
            from: Osmo::TICKER.into(),
            to: SwapTarget {
                pool_id: 1,
                target: Usdc::TICKER.to_owned(),
            },
        }];

        assert_eq!(expected, value);
    }
}
