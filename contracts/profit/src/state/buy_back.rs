use serde::{Deserialize, Serialize};

use currency::{
    native::{Native, Nls},
    payment::PaymentGroup,
};
use dex::{
    Account, CoinVisitor, Enterable, IterNext, IterState, Response as DexResponse, StateLocalOut,
    SwapTask,
};
use finance::{
    coin::CoinDTO,
    currency::{Currency as _, Symbol},
};
use oracle::stub::OracleRef;
use platform::{bank, batch::Batch, message::Response as PlatformResponse, never::Never};
use sdk::cosmwasm_std::{Addr, Env, QuerierWrapper};
use timealarms::stub::TimeAlarmsRef;

use crate::{error::ContractError, msg::ConfigResponse, profit::Profit, result::ContractResult};

use super::{idle::Idle, Config, ConfigManagement, SetupDexHandler, State, StateEnum};

#[derive(Serialize, Deserialize)]
pub(super) struct BuyBack {
    contract_addr: Addr,
    config: Config,
    account: Account,
    coins: Vec<CoinDTO<PaymentGroup>>,
}

impl BuyBack {
    pub fn new(
        contract_addr: Addr,
        config: Config,
        account: Account,
        coins: Vec<CoinDTO<PaymentGroup>>,
    ) -> Self {
        Self {
            contract_addr,
            config,
            account,
            coins,
        }
    }
}

impl SwapTask for BuyBack {
    type OutG = Native;
    type Label = String;
    type StateResponse = Never;
    type Result = ContractResult<DexResponse<State>>;

    fn label(&self) -> Self::Label {
        String::from("BuyBack")
    }

    fn dex_account(&self) -> &Account {
        &self.account
    }

    fn oracle(&self) -> &OracleRef {
        self.config.oracle()
    }

    fn time_alarm(&self) -> &TimeAlarmsRef {
        self.config.time_alarms()
    }

    fn out_currency(&self) -> Symbol<'_> {
        Nls::TICKER
    }

    fn on_coins<Visitor>(&self, visitor: &mut Visitor) -> Result<IterState, Visitor::Error>
    where
        Visitor: CoinVisitor<Result = IterNext>,
    {
        TryFind::try_find(&mut self.coins.iter(), |coin: &&CoinDTO<PaymentGroup>| {
            visitor
                .visit(coin)
                .map(|result: IterNext| matches!(result, IterNext::Stop))
        })
        .map(|maybe_coin: Option<&CoinDTO<PaymentGroup>>| {
            if maybe_coin.is_some() {
                IterState::Complete
            } else {
                IterState::Incomplete
            }
        })
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

        let state: Idle = Idle::new(self.config, self.account);

        let batch: Batch = state.enter(env.block.time, querier)?;

        Ok(DexResponse::<State> {
            response: PlatformResponse::messages_with_events(batch.merge(bank_batch), bank_emitter),
            next_state: State(StateEnum::Idle(state)),
        })
    }
}

impl ConfigManagement for StateLocalOut<BuyBack> {
    fn try_update_config(self, _: u16) -> ContractResult<Self> {
        Err(ContractError::UnsupportedOperation(String::from(
            "Configuration changes are not allowed during ICA opening process.",
        )))
    }

    fn try_query_config(&self) -> ContractResult<ConfigResponse> {
        Err(ContractError::UnsupportedOperation(String::from(
            "Querying configuration is not allowed during buy-back.",
        )))
    }
}

impl SetupDexHandler for StateLocalOut<BuyBack> {
    type State = Self;
}

trait TryFind
where
    Self: Iterator,
{
    fn try_find<F, E>(&mut self, mut f: F) -> Result<Option<Self::Item>, E>
    where
        F: FnMut(&Self::Item) -> Result<bool, E>,
    {
        for item in self {
            if f(&item)? {
                return Ok(Some(item));
            }
        }

        Ok(None)
    }
}

impl<I> TryFind for I where I: Iterator + ?Sized {}
