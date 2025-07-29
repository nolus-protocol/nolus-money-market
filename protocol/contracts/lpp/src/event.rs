use std::marker::PhantomData;

use currency::{CurrencyDef, platform::Nls};
use finance::coin::Coin;
use lpp_platform::NLpn;
use platform::batch::{Emit, Emitter};
use sdk::cosmwasm_std::{Addr, Env};

pub fn emit_deposit<Lpn>(
    env: Env,
    lender_addr: Addr,
    deposited_amount: Coin<Lpn>,
    receipts: Coin<NLpn>,
) -> Emitter
where
    Lpn: CurrencyDef,
{
    Emitter::of_type("lp-deposit")
        .emit_tx_info(&env)
        .emit("from", lender_addr)
        .emit("to", env.contract.address)
        .emit_coin("deposit", deposited_amount)
        .emit_coin_amount("receipts", receipts)
}

/// An events emitter supporting single and multiple withdrawals
pub(crate) struct WithdrawEmitter<'env, Lpn> {
    env: &'env Env,
    events: Vec<Emitter>,
    _lpn: PhantomData<Lpn>,
}

impl<'env, Lpn> WithdrawEmitter<'env, Lpn>
where
    Lpn: CurrencyDef,
{
    pub fn new(env: &'env Env) -> Self {
        Self {
            env,
            events: Default::default(),
            _lpn: Default::default(),
        }
    }

    pub fn on_withdraw(
        &mut self,
        lender: Addr,
        receipts: Coin<NLpn>,
        payment_out: Coin<Lpn>,
        may_reward: Option<Coin<Nls>>,
    ) {
        self.events.push(
            Emitter::of_type("lp-withdraw")
                .emit_tx_info(self.env)
                .emit("to", lender)
                .emit("from", self.env.contract.address.clone())
                .emit_coin("withdraw", payment_out)
                .emit_coin_amount("receipts", receipts)
                .emit_to_string_value("close", may_reward.is_some()),
        );
    }
}

impl<Lpn> IntoIterator for WithdrawEmitter<'_, Lpn> {
    type Item = Emitter;

    type IntoIter = <Vec<Self::Item> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.events.into_iter()
    }
}
