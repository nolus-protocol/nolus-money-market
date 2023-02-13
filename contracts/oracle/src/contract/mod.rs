use access_control::SingleUserAccess;
use currency::lpn::Lpns;
use finance::currency::{visit_any_on_ticker, AnyVisitor, AnyVisitorResult, Currency};
#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{from_binary, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply},
};
use versioning::{package_version, version, VersionSegment};

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

#[derive(serde::Serialize, serde::Deserialize)]
pub struct MigrateMsg {
    tree: tree::HumanReadableTree<swap::SwapTarget>,
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn migrate(deps: DepsMut<'_>, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    use sdk::cosmwasm_std::Storage;
    use swap::SwapTarget;
    use tree::HumanReadableTree;

    use crate::state::config::Config;

    struct UpdateTree<'r>(HumanReadableTree<SwapTarget>, &'r mut dyn Storage);

    impl<'r> AnyVisitor for UpdateTree<'r> {
        type Output = ();
        type Error = ContractError;

        fn on<C>(self) -> AnyVisitorResult<Self>
        where
            C: Currency + serde::Serialize + serde::de::DeserializeOwned,
        {
            SupportedPairs::<C>::new(self.0.into_tree())?
                .save(self.1)
                .map_err(Into::into)
        }
    }

    versioning::upgrade_old_contract::<0, _, ContractError>(
        deps.storage,
        version!(CONTRACT_STORAGE_VERSION),
        Some(|storage: &mut _| {
            visit_any_on_ticker::<Lpns, _>(
                &Config::load(storage)?.base_asset,
                UpdateTree(msg.tree, storage),
            )
        }),
    )?;

    Ok(Response::default())
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
    match msg {
        ExecuteMsg::UpdateConfig(price_config) => try_configure(deps.storage, info, price_config),
        ExecuteMsg::RegisterFeeder { feeder_address } => {
            Feeders::try_register(deps, info, feeder_address)
        }
        ExecuteMsg::RemoveFeeder { feeder_address } => {
            Feeders::try_remove(deps, info, feeder_address)
        }
        _ => Ok(ExecWithOracleBase::cmd(deps, env, msg, info.sender)?),
    }
}

// TODO: compare gas usage of this solution vs reply on error
#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn reply(deps: DepsMut<'_>, _env: Env, msg: Reply) -> Result<Response, ContractError> {
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
