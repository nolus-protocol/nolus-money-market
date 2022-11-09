use self::neutron::Module as NeutronModule;
#[cfg(not(feature = "neutron"))]
use cosmwasm_std::Empty as CustomMsg;
use cosmwasm_std::{
    testing::{MockApi, MockStorage},
    Empty,
};
use cw_multi_test::{BankKeeper, BasicAppBuilder, DistributionKeeper, StakeKeeper, WasmKeeper};
pub use cw_multi_test::{ContractWrapper, Executor};
#[cfg(feature = "neutron")]
use neutron_sdk::bindings::msg::NeutronMsg as CustomMsg;

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
>;

pub type Contract = dyn cw_multi_test::Contract<CustomMsg>;

pub fn new_app() -> AppBuilder {
    BasicAppBuilder::<CustomMsg, Empty>::new_custom()
        .with_custom(NeutronModule {})
        .with_wasm::<NeutronModule, _>(WasmKeeper::new())
}

mod neutron {
    use cosmwasm_schema::schemars::JsonSchema;
    use cosmwasm_std::{Addr, Api, Binary, BlockInfo, CustomQuery, Empty, Querier, Storage};

    use anyhow::{bail, Result as AnyResult};
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
