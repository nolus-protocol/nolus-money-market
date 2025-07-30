use std::result::Result as StdResult;

use currency::{CurrencyDTO, CurrencyDef, Group};
use finance::coin::{Coin, WithCoin, WithCoinResult};
use sdk::cosmwasm_std::{Addr, Coin as CwCoin, QuerierWrapper};

use crate::{
    bank::aggregate::{Aggregate, ReduceResults},
    coin_legacy,
    error::Error,
    result::Result,
};

pub type BalancesResult<G, Cmd> = StdResult<Option<WithCoinResult<G, Cmd>>, Error>;

pub trait BankAccountView {
    fn balance<C>(&self) -> Result<Coin<C>>
    where
        C: CurrencyDef;

    fn balances<G, Cmd>(&self, cmd: Cmd) -> BalancesResult<G, Cmd>
    where
        G: Group,
        Cmd: WithCoin<G> + Clone,
        Cmd::Output: Aggregate;
}

pub fn account_view<'a>(
    account: &'a Addr,
    querier: QuerierWrapper<'a>,
) -> impl BankAccountView + use<'a> {
    BankView::account(account, querier)
}

pub fn balance<'a, C>(account: &'a Addr, querier: QuerierWrapper<'a>) -> Result<Coin<C>>
where
    C: CurrencyDef,
{
    account_view(account, querier).balance()
}

pub struct BankView<'a> {
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
        Cmd::Output: Aggregate,
    {
        self.querier
            .query_all_balances(self.account)
            .map_err(Error::CosmWasmQueryAllBalances)
            .map(|cw_coins| {
                cw_coins
                    .into_iter()
                    .filter_map(|cw_coin| {
                        coin_legacy::maybe_from_cosmwasm_any::<G, _>(cw_coin, cmd.clone())
                    })
                    .reduce_results(Aggregate::aggregate)
            })
    }
}
