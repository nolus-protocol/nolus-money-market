use currency::lpn::Lpns;
use currency::{self, AnyVisitor, AnyVisitorResult, Currency};
use platform::{
    batch::{Emit, Emitter},
    response,
};
#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{
        to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Storage, SubMsgResult,
    },
};
use versioning::{package_version, version, VersionSegment};

use crate::{
    contract::alarms::MarketAlarms,
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, SudoMsg},
    result::ContractResult,
    state::{config::Config, supported_pairs::SupportedPairs},
};

use self::{
    config::query_config, exec::ExecWithOracleBase, oracle::feeder::Feeders,
    query::QueryWithOracleBase, sudo::SudoWithOracleBase,
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
}

impl<'a> InstantiateWithCurrency<'a> {
    pub fn cmd(
        deps: DepsMut<'a>,
        msg: InstantiateMsg,
    ) -> ContractResult<<Self as AnyVisitor>::Output> {
        let context = Self { deps, msg };
        currency::visit_any_on_ticker::<Lpns, _>(&context.msg.config.base_asset.clone(), context)
    }
}

impl<'a> AnyVisitor for InstantiateWithCurrency<'a> {
    type Output = ();
    type Error = ContractError;

    fn on<C>(self) -> AnyVisitorResult<Self>
    where
        C: Currency,
    {
        self.msg
            .config
            .store(self.deps.storage)
            .map_err(ContractError::StoreConfig)?;

        SupportedPairs::<C>::new(self.msg.swap_tree.into_tree())?
            .validate_tickers()?
            .save(self.deps.storage)?;

        Ok(())
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn instantiate(
    deps: DepsMut<'_>,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<CwResponse> {
    versioning::initialize(deps.storage, version!(CONTRACT_STORAGE_VERSION))
        .map_err(ContractError::InitializeVersioning)?;

    InstantiateWithCurrency::cmd(deps, msg)?;

    Ok(response::empty_response())
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn migrate(deps: DepsMut<'_>, _env: Env, _msg: MigrateMsg) -> ContractResult<CwResponse> {
    versioning::update_software(deps.storage, version!(CONTRACT_STORAGE_VERSION))
        .map_err(ContractError::UpdateSoftware)
        .and_then(response::response)
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn query(deps: Deps<'_>, env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::ContractVersion {} => {
            to_binary(&package_version!()).map_err(ContractError::ConvertToBinary)
        }
        QueryMsg::Config {} => {
            to_binary(&query_config(deps.storage)?).map_err(ContractError::ConvertToBinary)
        }
        QueryMsg::Feeders {} => Feeders::get(deps.storage)
            .map_err(ContractError::LoadFeeders)
            .and_then(|ref feeders| to_binary(feeders).map_err(ContractError::ConvertToBinary)),
        QueryMsg::IsFeeder { address } => Feeders::is_feeder(deps.storage, &address)
            .map_err(ContractError::LoadFeeders)
            .and_then(|ref f| to_binary(&f).map_err(ContractError::ConvertToBinary)),
        _ => QueryWithOracleBase::cmd(deps, env, msg),
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<CwResponse> {
    ExecWithOracleBase::cmd(deps, env, msg, info.sender)
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn sudo(deps: DepsMut<'_>, _env: Env, msg: SudoMsg) -> ContractResult<CwResponse> {
    match msg {
        SudoMsg::UpdateConfig(price_config) => Config::update(deps.storage, price_config),
        SudoMsg::RegisterFeeder { feeder_address } => Feeders::try_register(deps, feeder_address),
        SudoMsg::RemoveFeeder { feeder_address } => Feeders::try_remove(deps, feeder_address),
        _ => SudoWithOracleBase::cmd(deps, msg),
    }
    .map(|()| response::empty_response())
}

// TODO: compare gas usage of this solution vs reply on error
#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn reply(deps: DepsMut<'_>, _env: Env, msg: Reply) -> ContractResult<CwResponse> {
    const EVENT_TYPE: &str = "market-alarm";
    const KEY_DELIVERED: &str = "delivered";
    const KEY_DETAILS: &str = "details";

    let mut alarms: MarketAlarms<'_, &mut (dyn Storage + '_)> = MarketAlarms::new(deps.storage);

    let emitter: Emitter = Emitter::of_type(EVENT_TYPE);

    match msg.result {
        SubMsgResult::Ok(_) => alarms
            .last_delivered()
            .map(|()| emitter.emit(KEY_DELIVERED, "success")),
        SubMsgResult::Err(error) => alarms.last_failed().map(|()| {
            emitter
                .emit(KEY_DELIVERED, "error")
                .emit(KEY_DETAILS, error)
        }),
    }
    .map(response::response_only_messages)
}

#[cfg(test)]
mod tests {
    use currency::{lease::Osmo, lpn::Usdc, Currency};
    use finance::{duration::Duration, percent::Percent};
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
        let (deps, _info) = setup_test(msg);

        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!(
            ConfigResponse {
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
