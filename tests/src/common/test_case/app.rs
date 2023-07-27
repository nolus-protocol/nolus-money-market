use std::{cell::RefCell, fmt::Debug, rc::Rc};

use serde::Serialize;

use finance::duration::Duration;
use sdk::{
    cosmwasm_ext::{CosmosMsg, InterChainMsg},
    cosmwasm_std::{
        Addr, Api, Binary, BlockInfo, Coin as CwCoin, Empty, Querier, QuerierWrapper, Storage,
        WasmMsg, WasmQuery,
    },
    cw_multi_test::{
        AppResponse, Contract as CwContract, CosmosRouter, Executor, Wasm as WasmTrait, WasmKeeper,
    },
    testing::{CwApp, InterChainMsgReceiver},
};

use crate::common::{test_case::response::ResponseWithInterChainMsgs, AppExt as _};

pub(crate) trait Wasm: WasmTrait<InterChainMsg, Empty> + Sized {
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

impl<ActionSelectionF> WasmTrait<InterChainMsg, Empty> for ConfigurableWasm<ActionSelectionF>
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

#[must_use]
pub(crate) struct App<Wasm>
where
    Wasm: self::Wasm,
{
    app: CwApp<Wasm>,
    wasm_counter_part: Wasm::CounterPart,
    message_receiver: InterChainMsgReceiver,
}

impl<Wasm> App<Wasm>
where
    Wasm: self::Wasm,
{
    pub const fn new(
        app: CwApp<Wasm>,
        wasm_counter_part: Wasm::CounterPart,
        message_receiver: InterChainMsgReceiver,
    ) -> Self {
        Self {
            app,
            wasm_counter_part,
            message_receiver,
        }
    }

    #[must_use]
    pub fn store_code(&mut self, code: Box<dyn CwContract<InterChainMsg, Empty>>) -> u64 {
        Wasm::store_code(&mut self.app, &mut self.wasm_counter_part, code)
    }

    pub fn time_shift(&mut self, duration: Duration) {
        self.app.time_shift(duration)
    }

    pub fn update_block<F>(&mut self, f: F)
    where
        F: Fn(&mut BlockInfo),
    {
        self.app.update_block(f)
    }

    #[must_use]
    pub fn block_info(&self) -> BlockInfo {
        self.app.block_info()
    }

    pub fn send_tokens(
        &mut self,
        sender: Addr,
        recipient: Addr,
        amount: &[CwCoin],
    ) -> anyhow::Result<()> {
        self.app
            .send_tokens(sender, recipient, amount)
            .map(|_: AppResponse| ())
    }

    pub fn instantiate<'r, T, U>(
        &'r mut self,
        code_id: u64,
        sender: Addr,
        init_msg: &T,
        send_funds: &[CwCoin],
        label: U,
        admin: Option<String>,
    ) -> anyhow::Result<ResponseWithInterChainMsgs<'r, Addr>>
    where
        T: Debug + Serialize,
        U: Into<String>,
    {
        self.with_mock_app(|app: &mut CwApp<Wasm>| {
            app.instantiate_contract(code_id, sender, init_msg, send_funds, label, admin)
        })
    }

    pub fn execute<'r, T>(
        &'r mut self,
        sender: Addr,
        contract_addr: Addr,
        msg: &T,
        send_funds: &[CwCoin],
    ) -> anyhow::Result<ResponseWithInterChainMsgs<'r, AppResponse>>
    where
        T: Debug + Serialize,
    {
        self.with_mock_app(|app: &mut CwApp<Wasm>| {
            app.execute_contract(sender, contract_addr, msg, send_funds)
        })
    }

    pub fn execute_raw<T>(
        &mut self,
        sender: Addr,
        msg: T,
    ) -> anyhow::Result<ResponseWithInterChainMsgs<'_, AppResponse>>
    where
        T: Into<CosmosMsg>,
    {
        self.with_mock_app(|app: &mut CwApp<Wasm>| app.execute(sender, msg.into()))
    }

    pub fn sudo<'r, T, U>(
        &'r mut self,
        contract_addr: T,
        msg: &U,
    ) -> anyhow::Result<ResponseWithInterChainMsgs<'r, AppResponse>>
    where
        T: Into<Addr>,
        U: Serialize,
    {
        self.with_mock_app(|app: &mut CwApp<Wasm>| app.wasm_sudo(contract_addr, msg))
    }

    pub fn with_mock_app<F, R>(&mut self, f: F) -> anyhow::Result<ResponseWithInterChainMsgs<'_, R>>
    where
        F: FnOnce(&'_ mut CwApp<Wasm>) -> anyhow::Result<R>,
    {
        assert_eq!(self.message_receiver.try_recv().ok(), None);

        match f(&mut self.app) {
            Ok(result) => Ok(ResponseWithInterChainMsgs::new(
                &mut self.message_receiver,
                result,
            )),
            Err(error) => {
                // On error no messages should be "sent out".
                while self.message_receiver.try_iter().next().is_some() {}

                Err(error)
            }
        }
    }

    #[must_use]
    pub fn query(&self) -> QuerierWrapper<'_, Empty> {
        self.app.wrap()
    }
}
