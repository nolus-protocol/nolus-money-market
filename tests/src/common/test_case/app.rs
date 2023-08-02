use std::fmt::Debug;

use serde::Serialize;

use finance::duration::Duration;
use sdk::{
    cosmwasm_ext::{CosmosMsg, InterChainMsg},
    cosmwasm_std::{Addr, BlockInfo, Coin as CwCoin, Empty, QuerierWrapper},
    cw_multi_test::{AppResponse, Contract as CwContract, Executor},
    testing::{CwApp, InterChainMsgReceiver},
};

use crate::common::{test_case::response::ResponseWithInterChainMsgs, AppExt as _};

use super::wasm::Wasm as WasmTrait;

#[must_use]
pub(crate) struct App<Wasm>
where
    Wasm: WasmTrait,
{
    app: CwApp<Wasm>,
    wasm_counter_part: Wasm::CounterPart,
    message_receiver: InterChainMsgReceiver,
}

impl<Wasm> App<Wasm>
where
    Wasm: WasmTrait,
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
