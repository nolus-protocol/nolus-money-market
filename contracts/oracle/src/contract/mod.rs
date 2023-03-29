use access_control::SingleUserAccess;
use currency::lpn::Lpns;
use finance::currency::{visit_any_on_ticker, AnyVisitor, AnyVisitorResult, Currency};
use platform::response;
#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{
        from_binary, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Storage,
    },
};
use versioning::{package_version, version, VersionSegment};

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, SudoMsg},
    state::supported_pairs::SupportedPairs,
};

use self::{
    alarms::MarketAlarms,
    config::{query_config, try_configure},
    exec::ExecWithOracleBase,
    oracle::feeder::Feeders,
    query::QueryWithOracleBase,
    sudo::SudoWithOracleBase,
};

mod alarms;
mod config;
pub mod exec;
mod oracle;
pub mod query;
mod sudo;

// version info for migration info
// const CONTRACT_STORAGE_VERSION_FROM: VersionSegment = 0;
const CONTRACT_STORAGE_VERSION: VersionSegment = 0;

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

    fn on<C>(self) -> AnyVisitorResult<Self>
    where
        C: Currency,
    {
        SingleUserAccess::new_contract_owner(self.owner).store(self.deps.storage)?;

        self.msg.config.store(self.deps.storage)?;

        SupportedPairs::<C>::new(self.msg.swap_tree.into_tree())?
            .validate_tickers()?
            .save(self.deps.storage)?;

        Ok(Response::new().add_attribute("method", "instantiate"))
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn instantiate(
    deps: DepsMut<'_>,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    versioning::initialize(deps.storage, version!(CONTRACT_STORAGE_VERSION))?;

    InstantiateWithCurrency::cmd(deps, msg, info.sender)?;

    Ok(Response::default())
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn migrate(deps: DepsMut<'_>, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    versioning::update_software(deps.storage, version!(CONTRACT_STORAGE_VERSION))?;

    SingleUserAccess::remove_contract_owner(deps.storage);

    response::response(versioning::release()).map_err(Into::into)
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn query(deps: Deps<'_>, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::ContractVersion {} => Ok(to_binary(&package_version!())?),
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
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    ExecWithOracleBase::cmd(deps, env, msg, info.sender)
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn sudo(deps: DepsMut<'_>, _env: Env, msg: SudoMsg) -> Result<Response, ContractError> {
    match msg {
        SudoMsg::UpdateConfig(price_config) => try_configure(deps.storage, price_config),
        SudoMsg::RegisterFeeder { feeder_address } => Feeders::try_register(deps, feeder_address),
        SudoMsg::RemoveFeeder { feeder_address } => Feeders::try_remove(deps, feeder_address),
        SudoMsg::RemovePriceAlarm { receiver } => {
            MarketAlarms::remove(deps.storage, receiver)?;

            Ok(Response::default())
        }
        _ => SudoWithOracleBase::cmd(deps, msg),
    }
}

// TODO: compare gas usage of this solution vs reply on error
#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn reply(deps: DepsMut<'_>, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let resp = match msg.result {
        cosmwasm_std::SubMsgResult::Ok(resp) => match resp.data {
            Some(d) => on_success_alarm(deps.storage, d)?,
            None => on_err_alarm(
                "Market alarm receiver's contract not respected! No receiver response!",
            ),
        },
        cosmwasm_std::SubMsgResult::Err(err) => on_err_alarm(err),
    };
    Ok(resp)
}

fn on_success_alarm(storage: &mut dyn Storage, resp: Binary) -> Result<Response, ContractError> {
    MarketAlarms::remove(storage, from_binary(&resp)?)?;
    Ok(Response::new().add_attribute("alarm", "success"))
}

fn on_err_alarm<S>(err: S) -> Response
where
    S: Into<String>,
{
    Response::new()
        .add_attribute("alarm", "error")
        .add_attribute("error", err)
}

#[cfg(test)]
mod tests {
    use currency::{lease::Osmo, lpn::Usdc};
    use finance::{currency::Currency, duration::Duration, percent::Percent};
    use sdk::cosmwasm_std::{from_binary, testing::mock_env};
    use swap::SwapTarget;

    use crate::{
        contract::query,
        msg::{ConfigResponse, QueryMsg},
        state::{config::Config, supported_pairs::SwapLeg},
        swap_tree,
        tests::{dummy_instantiate_msg, setup_test},
    };

    #[test]
    fn proper_initialization() {
        use marketprice::config::Config as PriceConfig;
        let msg = dummy_instantiate_msg(
            Usdc::TICKER.to_string(),
            60,
            Percent::from_percent(50),
            swap_tree!((1, Osmo::TICKER)),
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
                        Percent::from_percent(50),
                        Duration::from_secs(60),
                        1,
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
                target: Usdc::TICKER.into(),
            },
        }];

        assert_eq!(expected, value);
    }
}
