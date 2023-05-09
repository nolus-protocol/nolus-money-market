use currency::native::Nls;
use finance::coin::Coin;
use platform::{
    bank::BankAccount,
    batch::{Batch, Emit as _, Emitter},
};
use sdk::cosmwasm_std::{Addr, Env, Storage};

use crate::{
    msg::ConfigResponse,
    result::ContractResult,
    state::{ConfigManagement as _, State},
};

pub struct Profit;

impl Profit {
    pub(crate) fn transfer_nls<B>(
        mut account: B,
        env: &Env,
        treasury_addr: &Addr,
    ) -> ContractResult<(Batch, Emitter)>
    where
        B: BankAccount,
    {
        let balance_nls: Coin<Nls> = account.balance()?;

        account.send(balance_nls, treasury_addr);

        Ok((
            account.into(),
            Emitter::of_type("tr-profit")
                .emit_tx_info(env)
                .emit_coin("profit-amount", balance_nls),
        ))
    }

    pub fn query_config(storage: &dyn Storage) -> ContractResult<ConfigResponse> {
        State::load(storage).and_then(|state: State| state.try_query_config())
    }
}
