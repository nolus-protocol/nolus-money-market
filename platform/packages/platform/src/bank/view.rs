use std::{cell::OnceCell, result::Result as StdResult};

use currency::{CurrencyDTO, CurrencyDef, Group};
use finance::coin::{Coin, WithCoin};
use sdk::cosmwasm_std::{Addr, Coin as CwCoin, QuerierWrapper, Uint128};

use crate::{
    bank::aggregate::{Aggregate, ReduceResults},
    coin_legacy,
    error::Error,
    result::Result,
};

pub type BalancesResult<G, Cmd> = StdResult<Option<<Cmd as WithCoin<G>>::Outcome>, Error>;

pub trait BankAccountView {
    fn balance<C>(&self) -> Result<Coin<C>>
    where
        C: CurrencyDef;

    /// Return only the non-zero balances of all currencies belonging to the group
    fn balances<G, Cmd>(&self, cmd: Cmd) -> BalancesResult<G, Cmd>
    where
        G: Group,
        Cmd: WithCoin<G> + Clone,
        Cmd::Outcome: Aggregate;
}

pub fn account_view<'a>(
    account: &'a Addr,
    querier: QuerierWrapper<'a>,
) -> impl BankAccountView + use<'a> {
    BankView::account(account, querier)
}

pub fn cache<C, View>(view: View) -> impl BankAccountView
where
    C: CurrencyDef,
    View: BankAccountView,
{
    BalanceCache::<C, _>::new(view)
}

pub fn balance<'a, C>(account: &'a Addr, querier: QuerierWrapper<'a>) -> Result<Coin<C>>
where
    C: CurrencyDef,
{
    account_view(account, querier).balance()
}

struct BankView<'a> {
    account: &'a Addr,
    querier: QuerierWrapper<'a>,
}

impl<'a> BankView<'a> {
    fn account(account: &'a Addr, querier: QuerierWrapper<'a>) -> Self {
        Self { account, querier }
    }

    fn cw_balance<G>(&self, currency: &CurrencyDTO<G>) -> Result<CwCoin>
    where
        G: Group,
    {
        self.querier
            .query_balance(self.account, currency.definition().bank_symbol)
            .map_err(Error::CosmWasmQueryBalance)
    }
}

impl BankAccountView for BankView<'_> {
    fn balance<C>(&self) -> Result<Coin<C>>
    where
        C: CurrencyDef,
    {
        self.cw_balance(C::dto()).and_then(|ref cw_coin| {
            coin_legacy::from_cosmwasm_currency_not_definition::<C, _>(cw_coin)
        })
    }

    fn balances<G, Cmd>(&self, cmd: Cmd) -> BalancesResult<G, Cmd>
    where
        G: Group,
        Cmd: WithCoin<G> + Clone,
        Cmd::Outcome: Aggregate,
    {
        G::currencies()
            .filter_map(|ref currency| {
                self.cw_balance(currency).map_err(Into::into).map_or_else(
                    |err| Some(Err(err)),
                    |ref cw_balance| {
                        if cw_balance.amount == Uint128::zero() {
                            None
                        } else {
                            Some(coin_legacy::from_cosmwasm_any::<G, _>(
                                cw_balance,
                                cmd.clone(),
                            ))
                        }
                    },
                )
            })
            .reduce_results(Aggregate::aggregate)
            .transpose()
    }
}

struct BalanceCache<C, View> {
    cache: OnceCell<Coin<C>>,
    view: View,
}

impl<C, View> BalanceCache<C, View>
where
    View: BankAccountView,
{
    fn new(view: View) -> Self {
        Self {
            cache: OnceCell::new(),
            view,
        }
    }
}

impl<C, View> BankAccountView for BalanceCache<C, View>
where
    C: CurrencyDef,
    View: BankAccountView,
{
    fn balance<CC>(&self) -> Result<Coin<CC>>
    where
        CC: CurrencyDef,
    {
        if currency::equal::<C, CC>() {
            // cannot use OnceCell::get_or_init since getting an error should not set any value
            // and OnceCell::get_or_try_init is unstable at this point
            match self.cache.get() {
                Some(cache) => Ok(cache.coerce_into()),
                None => self
                    .view
                    .balance()
                    .inspect(|val| {
                        let set_result = self.cache.set(*val);
                        debug_assert_eq!(Ok(()), set_result);
                    })
                    .map(|val| val.coerce_into()),
            }
        } else {
            self.view.balance()
        }
    }

    fn balances<G, Cmd>(&self, cmd: Cmd) -> BalancesResult<G, Cmd>
    where
        G: Group,
        Cmd: WithCoin<G> + Clone,
        Cmd::Outcome: Aggregate,
    {
        self.view.balances(cmd)
    }
}

#[cfg(test)]
mod test {
    use currency::test::{SuperGroupTestC1, SuperGroupTestC2};
    use finance::coin::Coin;

    use crate::bank::{BankAccountView, testing};

    const BALANCE1: Coin<SuperGroupTestC1> = Coin::new(242);
    const BALANCE2: Coin<SuperGroupTestC2> = Coin::new(3412);

    #[test]
    fn cache_view_ok() {
        let view = testing::MockBankView::new(BALANCE1, BALANCE2);
        let cache = super::cache::<SuperGroupTestC1, _>(view);
        assert_eq!(Ok(BALANCE1), cache.balance());
        assert_eq!(Ok(BALANCE2), cache.balance());
        assert_eq!(Ok(BALANCE2), cache.balance());
    }

    #[test]
    fn query_once() {
        let view = testing::take_balance_once(BALANCE1, testing::not_taking_balance());
        let cache = super::cache::<SuperGroupTestC1, _>(view);
        assert_eq!(Ok(BALANCE1), cache.balance());
        assert_eq!(Ok(BALANCE1), cache.balance());
        assert_eq!(Ok(BALANCE1), cache.balance());
    }

    #[test]
    fn query_once_two_currencies() {
        let view = testing::take_balance_once(
            BALANCE1,
            testing::take_balance_once(BALANCE2, testing::not_taking_balance()),
        );
        let cache = super::cache::<SuperGroupTestC1, _>(super::cache::<SuperGroupTestC2, _>(view));
        assert_eq!(Ok(BALANCE1), cache.balance());
        assert_eq!(Ok(BALANCE2), cache.balance());
        assert_eq!(Ok(BALANCE2), cache.balance());
        assert_eq!(Ok(BALANCE1), cache.balance());
    }
}
