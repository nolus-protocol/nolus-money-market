#[cfg(not(feature = "neutron"))]
use cosmwasm_std::Empty as CustomMsg;
use cosmwasm_std::{
    testing::{mock_dependencies, MockApi, MockQuerier, MockStorage},
    Binary, ContractResult, Empty, GovMsg, IbcMsg, IbcQuery, OwnedDeps, SystemError, SystemResult,
    WasmQuery,
};
use cw_multi_test::{
    BankKeeper, BasicAppBuilder, DistributionKeeper, FailingModule, StakeKeeper, WasmKeeper,
};
pub use cw_multi_test::{ContractWrapper, Executor};
#[cfg(feature = "neutron")]
use neutron_sdk::bindings::msg::NeutronMsg as CustomMsg;

use self::neutron::Module as NeutronModule;

pub type App<Exec = CustomMsg, Query = Empty> =
    cw_multi_test::App<BankKeeper, MockApi, MockStorage, NeutronModule, WasmKeeper<Exec, Query>>;

pub type AppBuilder<Exec = CustomMsg, Query = Empty> = cw_multi_test::AppBuilder<
    BankKeeper,
    MockApi,
    MockStorage,
    NeutronModule,
    WasmKeeper<Exec, Query>,
    StakeKeeper,
    DistributionKeeper,
    FailingModule<IbcMsg, IbcQuery, Empty>,
    FailingModule<GovMsg, Empty, Empty>,
>;

pub type Contract = dyn cw_multi_test::Contract<CustomMsg>;

pub fn mock_deps_with_contracts<const N: usize>(
    contracts: [&'static str; N],
) -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    customized_mock_deps_with_contracts(mock_dependencies(), contracts)
}

pub fn customized_mock_deps_with_contracts<const N: usize>(
    mut deps: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    contracts: [&'static str; N],
) -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    deps.querier.update_wasm(move |query| match query {
        WasmQuery::ContractInfo { contract_addr }
            if contracts.contains(&contract_addr.as_str()) =>
        {
            SystemResult::Ok(ContractResult::Ok(Binary(Vec::from(
                br#"{"code_id":0,"creator":"","admin":null,"pinned":false,"ibc_port":null}"#
                    as &[u8],
            ))))
        }
        WasmQuery::Smart { contract_addr, .. }
        | WasmQuery::Raw { contract_addr, .. }
        | WasmQuery::ContractInfo { contract_addr, .. } => {
            SystemResult::Err(SystemError::NoSuchContract {
                addr: contract_addr.clone(),
            })
        }
        _ => unimplemented!(),
    });

    deps
}

pub fn new_app() -> AppBuilder {
    BasicAppBuilder::<CustomMsg, Empty>::new_custom()
        .with_custom(NeutronModule {})
        .with_wasm::<NeutronModule, _>(WasmKeeper::new())
}

mod neutron {
    use anyhow::{bail, Result as AnyResult};
    use cosmwasm_schema::schemars::JsonSchema;
    use cosmwasm_std::{Addr, Api, Binary, BlockInfo, CustomQuery, Empty, Querier, Storage};
    use cw_multi_test::{AppResponse, CosmosRouter, Module as CwModule};
    use neutron_sdk::bindings::msg::NeutronMsg;
    use serde::de::DeserializeOwned;

    pub struct Module {}
    impl CwModule for Module {
        type ExecT = NeutronMsg;

        type QueryT = Empty;

        type SudoT = Empty;

        fn execute<ExecC, QueryC>(
            &self,
            _api: &dyn Api,
            _storage: &mut dyn Storage,
            _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
            _block: &BlockInfo,
            _sender: Addr,
            _msg: Self::ExecT,
        ) -> AnyResult<AppResponse>
        where
            ExecC: std::fmt::Debug
                + Clone
                + PartialEq
                + JsonSchema
                + serde::de::DeserializeOwned
                + 'static,
            QueryC: CustomQuery + DeserializeOwned + 'static,
        {
            Ok(AppResponse {
                ..Default::default()
            })
        }

        fn sudo<ExecC, QueryC>(
            &self,
            _api: &dyn Api,
            _storage: &mut dyn Storage,
            _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
            _block: &BlockInfo,
            msg: Self::SudoT,
        ) -> AnyResult<AppResponse>
        where
            ExecC: std::fmt::Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
            QueryC: CustomQuery + DeserializeOwned + 'static,
        {
            bail!("Unexpected sudo msg {:?}", msg)
        }

        fn query(
            &self,
            _api: &dyn Api,
            _storage: &dyn Storage,
            _querier: &dyn Querier,
            _block: &BlockInfo,
            request: Self::QueryT,
        ) -> AnyResult<Binary> {
            bail!("Unexpected custom query {:?}", request)
        }
    }
}
