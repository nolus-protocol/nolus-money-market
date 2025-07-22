use currency::{CurrencyDef, Group};
use finance::coin::{Coin, WithCoin};

use crate::{
    bank::{Aggregate, BalancesResult, BankAccountView},
    result::Result,
};

pub struct MockBankView<OneC, OtherC> {
    balance: Coin<OneC>,
    balance_other: Coin<OtherC>,
}

impl<OneC, OtherC> MockBankView<OneC, OtherC> {
    pub fn new(balance: Coin<OneC>, balance_other: Coin<OtherC>) -> Self {
        Self {
            balance,
            balance_other,
        }
    }
    pub fn only_balance(balance: Coin<OneC>) -> Self {
        Self {
            balance,
            balance_other: Coin::default(),
        }
    }
}

impl<OneC, OtherC> BankAccountView for MockBankView<OneC, OtherC>
where
    OneC: CurrencyDef,
    OtherC: CurrencyDef,
{
    fn balance<C>(&self) -> Result<Coin<C>>
    where
        C: CurrencyDef,
    {
        if currency::equal::<C, OneC>() {
            Ok(Coin::<C>::new(self.balance.into()))
        } else if currency::equal::<C, OtherC>() {
            Ok(Coin::<C>::new(self.balance_other.into()))
        } else {
            unreachable!(
                "Expected {} or {}, found {}",
                currency::to_string(OneC::dto()),
                currency::to_string(OtherC::dto()),
                currency::to_string(C::dto())
            );
        }
    }

    fn balances<G, Cmd>(&self, _: Cmd) -> BalancesResult<G, Cmd>
    where
        G: Group,
        Cmd: WithCoin<G>,
        Cmd::Output: Aggregate,
    {
        unimplemented!()
    }
}
