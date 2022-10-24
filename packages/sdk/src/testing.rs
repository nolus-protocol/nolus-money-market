#![cfg(not(target_arch = "wasm32"))]

use std::fmt::Debug;

use cosmwasm_schema::schemars::JsonSchema;
#[cfg(not(feature = "neutron"))]
use cosmwasm_std::Empty as CustomMsg;
use cosmwasm_std::{
    testing::{MockApi, MockStorage},
    CustomQuery, Empty,
};
use cw_multi_test::{
    BankKeeper, DistributionKeeper, FailingModule, Module, StakeKeeper, Wasm, WasmKeeper,
};
pub use cw_multi_test::{ContractWrapper, Executor};
#[cfg(feature = "neutron")]
use neutron_sdk::bindings::msg::NeutronMsg as CustomMsg;
use serde::de::DeserializeOwned;

pub type App<Exec = CustomMsg, Query = Empty, Sudo = Empty, Api = MockApi> = cw_multi_test::App<
    BankKeeper,
    Api,
    MockStorage,
    FailingModule<Exec, Query, Sudo>,
    WasmKeeper<Exec, Query>,
>;

pub type AppBuilder<Exec = CustomMsg, Query = Empty, Sudo = Empty> = cw_multi_test::AppBuilder<
    BankKeeper,
    MockApi,
    MockStorage,
    FailingModule<Exec, Query, Sudo>,
    WasmKeeper<Exec, Query>,
    StakeKeeper,
    DistributionKeeper,
>;

pub type Contract = dyn cw_multi_test::Contract<CustomMsg>;

pub fn new_app<Exec, Query, Sudo>() -> AppBuilder<Exec, Query, Sudo>
where
    Exec: Debug + PartialEq + JsonSchema + DeserializeOwned + Clone + 'static,
    Query: Debug + CustomQuery + DeserializeOwned + 'static,
    Sudo: Debug,
    FailingModule<Exec, Query, Sudo>: Module<ExecT = Exec, QueryT = Query, SudoT = Sudo>,
    WasmKeeper<Exec, Query>: Wasm<Exec, Query>,
{
    AppBuilder::new()
        .with_custom(FailingModule::<Exec, Query, Sudo>::new())
        .with_wasm::<FailingModule<Exec, Query, Sudo>, _>(WasmKeeper::<Exec, Query>::new())
}
