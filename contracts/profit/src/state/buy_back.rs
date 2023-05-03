use serde::{Deserialize, Serialize};

use currency::{
    native::{Native, Nls},
    payment::PaymentGroup,
};
use dex::{
    Account, CoinVisitor, Enterable, Error as DexError, IterNext, IterState,
    Response as DexResponse, StateLocalOut, SwapTask,
};
use finance::{
    coin::CoinDTO,
    currency::{Currency as _, Symbol},
};
use oracle::stub::OracleRef;
use platform::{
    bank::{self},
    batch::Batch,
    message::Response as PlatformResponse,
};
use sdk::cosmwasm_std::{Addr, Env, QuerierWrapper};
use timealarms::stub::TimeAlarmsRef;

use crate::profit::Profit;

use super::{idle::Idle, Config, ProfitMessageHandler, State, StateEnum};

#[derive(Serialize, Deserialize)]
pub(super) struct BuyBack {
    contract_addr: Addr,
    config: Config,
    account: Account,
    oracle: OracleRef,
    time_alarms: TimeAlarmsRef,
    coins: Vec<CoinDTO<PaymentGroup>>,
}

impl BuyBack {
    pub fn new(
        contract_addr: Addr,
        config: Config,
        account: Account,
        oracle: OracleRef,
        time_alarms: TimeAlarmsRef,
        coins: Vec<CoinDTO<PaymentGroup>>,
    ) -> Self {
        Self {
            contract_addr,
            config,
            account,
            oracle,
            time_alarms,
            coins,
        }
    }
}

impl SwapTask for BuyBack {
    type OutG = Native;
    type Label = String;
    type StateResponse = State;
    type Result = Result<DexResponse<State>, DexError>;

    fn label(&self) -> Self::Label {
        String::from("BuyBack")
    }

    fn dex_account(&self) -> &Account {
        &self.account
    }

    fn oracle(&self) -> &OracleRef {
        &self.oracle
    }

    fn time_alarm(&self) -> &TimeAlarmsRef {
        &self.time_alarms
    }

    fn out_currency(&self) -> Symbol<'_> {
        Nls::DEX_SYMBOL
    }

    fn on_coins<Visitor>(&self, visitor: &mut Visitor) -> Result<IterState, Visitor::Error>
    where
        Visitor: CoinVisitor<Result = IterNext>,
    {
        let mut early_exit: bool = false;

        for coin in &self.coins {
            if early_exit {
                return Ok(IterState::Incomplete);
            }

            if matches!(visitor.visit(coin)?, IterNext::Stop) {
                early_exit = true;
            }
        }

        Ok(IterState::Complete)
    }

    fn finish(
        self,
        _: CoinDTO<Self::OutG>,
        env: &Env,
        querier: &QuerierWrapper<'_>,
    ) -> Self::Result {
        let (bank_batch, bank_emitter) = Profit::transfer_nls(
            bank::account(&self.contract_addr, querier),
            env,
            self.config.treasury(),
        )?;

        let state: Idle = Idle::new(self.config, self.account, self.oracle, self.time_alarms);

        let batch: Batch = state.enter(env.block.time, querier)?;

        Ok(DexResponse::<State> {
            response: PlatformResponse::messages_with_events(batch.merge(bank_batch), bank_emitter),
            next_state: State(StateEnum::Idle(state)),
        })
    }
}

impl ProfitMessageHandler for StateLocalOut<BuyBack> {}
