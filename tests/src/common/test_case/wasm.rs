use std::{cell::RefCell, rc::Rc};

use sdk::{
    cosmwasm_std::{Addr, Api, Binary, BlockInfo, Empty, Querier, Storage, WasmMsg, WasmQuery},
    cw_multi_test::{
        AppResponse, Contract as CwContract, CosmosRouter, Wasm as CwWasm, WasmKeeper,
    },
    neutron_sdk::bindings::msg::NeutronMsg as InterChainMsg,
    testing::CwApp,
};

pub(crate) trait Wasm: CwWasm<InterChainMsg, Empty> + Sized {
    type CounterPart;

    fn store_code(
        app: &mut CwApp<Self>,
        counter_part: &mut Self::CounterPart,
        code: Box<dyn CwContract<InterChainMsg, Empty>>,
    ) -> u64;
}

pub(crate) type DefaultWasm = WasmKeeper<InterChainMsg, Empty>;

impl Wasm for DefaultWasm {
    type CounterPart = ();

    fn store_code(
        app: &mut CwApp<Self>,
        &mut (): &mut Self::CounterPart,
        code: Box<dyn CwContract<InterChainMsg, Empty>>,
    ) -> u64 {
        app.store_code(code)
    }
}

pub(crate) fn default_wasm() -> (DefaultWasm, ()) {
    (DefaultWasm::new(), ())
}

pub(crate) enum Request<'r> {
    Query(&'r WasmQuery),
    Execute(&'r WasmMsg),
    Sudo(&'r Addr),
}

pub(crate) enum Action {
    Forward,
    Error(anyhow::Error),
}

pub(crate) struct ConfigurableWasmBuilder<ActionSelectionF>
where
    ActionSelectionF: Fn(Request<'_>) -> Action,
{
    action_selection: ActionSelectionF,
}

impl<ActionSelectionF> ConfigurableWasmBuilder<ActionSelectionF>
where
    ActionSelectionF: Fn(Request<'_>) -> Action,
{
    pub const fn new(action_selection: ActionSelectionF) -> Self {
        Self { action_selection }
    }

    pub fn build(
        self,
    ) -> (
        ConfigurableWasm<ActionSelectionF>,
        <ConfigurableWasm<ActionSelectionF> as Wasm>::CounterPart,
    ) {
        let inner: Rc<RefCell<DefaultWasm>> = Default::default();

        (
            ConfigurableWasm {
                inner: inner.clone(),
                action_selection: self.action_selection,
            },
            inner,
        )
    }
}

pub(crate) struct ConfigurableWasm<ActionSelectionF>
where
    ActionSelectionF: Fn(Request<'_>) -> Action,
{
    inner: Rc<RefCell<DefaultWasm>>,
    action_selection: ActionSelectionF,
}

impl<ActionSelectionF> CwWasm<InterChainMsg, Empty> for ConfigurableWasm<ActionSelectionF>
where
    ActionSelectionF: Fn(Request<'_>) -> Action,
{
    fn query(
        &self,
        api: &dyn Api,
        storage: &dyn Storage,
        querier: &dyn Querier,
        block: &BlockInfo,
        request: WasmQuery,
    ) -> anyhow::Result<Binary> {
        match (self.action_selection)(Request::Query(&request)) {
            Action::Forward => self
                .inner
                .borrow()
                .query(api, storage, querier, block, request),
            Action::Error(error) => Err(error),
        }
    }

    fn execute(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = InterChainMsg, QueryC = Empty>,
        block: &BlockInfo,
        sender: Addr,
        msg: WasmMsg,
    ) -> anyhow::Result<AppResponse> {
        match (self.action_selection)(Request::Execute(&msg)) {
            Action::Forward => self
                .inner
                .borrow()
                .execute(api, storage, router, block, sender, msg),
            Action::Error(error) => Err(error),
        }
    }

    fn sudo(
        &self,
        api: &dyn Api,
        contract_addr: Addr,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = InterChainMsg, QueryC = Empty>,
        block: &BlockInfo,
        msg: Binary,
    ) -> anyhow::Result<AppResponse> {
        match (self.action_selection)(Request::Sudo(&contract_addr)) {
            Action::Forward => {
                self.inner
                    .borrow()
                    .sudo(api, contract_addr, storage, router, block, msg)
            }
            Action::Error(error) => Err(error),
        }
    }
}

impl<ActionSelectionF> Wasm for ConfigurableWasm<ActionSelectionF>
where
    ActionSelectionF: Fn(Request<'_>) -> Action,
{
    type CounterPart = Rc<RefCell<DefaultWasm>>;

    fn store_code(
        _: &mut CwApp<Self>,
        counter_part: &mut Self::CounterPart,
        code: Box<dyn CwContract<InterChainMsg, Empty>>,
    ) -> u64 {
        counter_part
            .borrow_mut()
            .store_code(code)
            .try_into()
            .unwrap()
    }
}
