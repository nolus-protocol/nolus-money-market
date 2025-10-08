use std::fmt::Debug;

use anyhow::anyhow;
use serde::Serialize;

use finance::duration::Duration;
use platform::contract::Code;
use sdk::{
    cosmwasm_ext::{CosmosMsg, InterChainMsg},
    cosmwasm_std::{Addr, BlockInfo, Coin as CwCoin, Empty, QuerierWrapper},
    cw_multi_test::{AppResponse, Contract as CwContract, Executor},
    testing::InterChainMsgReceiver,
};

use crate::common::{AppExt as _, MockApp, test_case::response::ResponseWithInterChainMsgs};

#[must_use]
pub(crate) struct App {
    app: MockApp,
    message_receiver: InterChainMsgReceiver,
}

impl App {
    pub const fn new(app: MockApp, message_receiver: InterChainMsgReceiver) -> Self {
        Self {
            app,
            message_receiver,
        }
    }

    #[must_use]
    pub fn store_code(&mut self, code: Box<dyn CwContract<InterChainMsg, Empty>>) -> Code {
        Code::unchecked(self.app.store_code(code))
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
            .map_err(|error| anyhow!(error))
    }

    pub fn instantiate<'r, T, U>(
        &'r mut self,
        code: Code,
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
        self.with_mock_app(|app: &mut MockApp| {
            app.instantiate_contract(code.into(), sender, init_msg, send_funds, label, admin)
        })
        .map_err(|error| anyhow!(error))
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
        self.with_mock_app(|app: &mut MockApp| {
            app.execute_contract(sender, contract_addr, msg, send_funds)
        })
        .map_err(|error| anyhow!(error))
    }

    pub fn execute_raw<T>(
        &mut self,
        sender: Addr,
        msg: T,
    ) -> anyhow::Result<ResponseWithInterChainMsgs<'_, AppResponse>>
    where
        T: Into<CosmosMsg>,
    {
        self.with_mock_app(|app: &mut MockApp| app.execute(sender, msg.into()))
            .map_err(|error| anyhow!(error))
    }

    pub fn migrate<'r, T>(
        &'r mut self,
        sender: Addr,
        contract_addr: Addr,
        msg: &T,
        new_code_id: u64,
    ) -> anyhow::Result<ResponseWithInterChainMsgs<'r, AppResponse>>
    where
        T: Serialize,
    {
        self.with_mock_app(|app: &mut MockApp| {
            app.migrate_contract(sender, contract_addr, msg, new_code_id)
        })
        .map_err(|error| anyhow!(error))
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
        self.with_mock_app(|app: &mut MockApp| app.wasm_sudo(contract_addr, msg))
            .map_err(|error| anyhow!(error))
    }

    pub fn with_mock_app<F, R, E>(&mut self, f: F) -> Result<ResponseWithInterChainMsgs<'_, R>, E>
    where
        F: FnOnce(&'_ mut MockApp) -> Result<R, E>,
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
