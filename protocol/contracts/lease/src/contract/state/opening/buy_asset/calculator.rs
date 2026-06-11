use std::marker::PhantomData;

use currency::{AnyVisitor, CurrencyDTO, CurrencyDef, Group, MemberOf};
use dex::{Error as DexError, MaxSlippage, SlippageCalculator, SwapTask, WithCalculator};
use finance::coin::{Amount, Coin, CoinDTO, WithCoin};
use oracle::stub;
use sdk::cosmwasm_std::QuerierWrapper;

use crate::{
    api::{LeaseAssetCurrencies, LeasePaymentCurrencies},
    finance::{LpnCurrencies, LpnCurrency, OracleRef},
};

use super::BuyAsset;

pub struct Factory<'oracle, WithCalc> {
    with_calc: WithCalc,
    max_slippage: MaxSlippage,
    oracle: &'oracle OracleRef,
}

struct MinOutput<'oracle, OutC> {
    max_slippage: MaxSlippage,
    oracle: &'oracle OracleRef,
    _out: PhantomData<OutC>,
}

struct ToAsset<'oracle, OutC> {
    oracle: &'oracle OracleRef,
    querier: QuerierWrapper<'oracle>,
    _out: PhantomData<OutC>,
}

impl<'oracle, WithCalc> Factory<'oracle, WithCalc> {
    pub const fn new(
        with_calc: WithCalc,
        max_slippage: MaxSlippage,
        oracle: &'oracle OracleRef,
    ) -> Self {
        Self {
            with_calc,
            max_slippage,
            oracle,
        }
    }
}

impl<WithCalc> AnyVisitor<<BuyAsset as SwapTask>::OutG> for Factory<'_, WithCalc>
where
    WithCalc: WithCalculator<BuyAsset>,
{
    type Outcome = WithCalc::Output;

    fn on<C>(self, _def: &CurrencyDTO<C::Group>) -> Self::Outcome
    where
        C: CurrencyDef,
        C::Group: MemberOf<<BuyAsset as SwapTask>::OutG>
            + MemberOf<<<BuyAsset as SwapTask>::OutG as Group>::TopG>,
    {
        self.with_calc.on(&MinOutput::<'_, C> {
            max_slippage: self.max_slippage,
            oracle: self.oracle,
            _out: PhantomData,
        })
    }
}

impl<OutC> SlippageCalculator<LeasePaymentCurrencies> for MinOutput<'_, OutC>
where
    OutC: CurrencyDef,
    OutC::Group: MemberOf<LeaseAssetCurrencies> + MemberOf<<LeasePaymentCurrencies as Group>::TopG>,
{
    type OutC = OutC;

    fn min_output(
        &self,
        input: &CoinDTO<LeasePaymentCurrencies>,
        querier: QuerierWrapper<'_>,
    ) -> dex::DexResult<Coin<Self::OutC>> {
        // A dust input may quote to a zero output which the swap params
        // reject; the floor of one mirrors `AcceptAnyNonZeroSwap`.
        const MIN_FLOOR: Amount = 1;

        input
            .with_coin(ToAsset::<'_, OutC> {
                oracle: self.oracle,
                querier,
                _out: PhantomData,
            })
            .map(|quote| self.max_slippage.min_out(quote).max(Coin::new(MIN_FLOOR)))
    }
}

impl<OutC> WithCoin<LeasePaymentCurrencies> for ToAsset<'_, OutC>
where
    OutC: CurrencyDef,
    OutC::Group: MemberOf<LeaseAssetCurrencies> + MemberOf<<LeasePaymentCurrencies as Group>::TopG>,
{
    type Outcome = dex::DexResult<Coin<OutC>>;

    fn on<C>(self, input: Coin<C>) -> Self::Outcome
    where
        C: CurrencyDef,
        C::Group:
            MemberOf<LeasePaymentCurrencies> + MemberOf<<LeasePaymentCurrencies as Group>::TopG>,
    {
        stub::to_quote::<C, LeasePaymentCurrencies, LpnCurrency, LpnCurrencies>(
            self.oracle.clone(),
            input,
            self.querier,
        )
        .and_then(|in_lpn| {
            stub::from_quote::<LpnCurrency, LpnCurrencies, OutC, LeaseAssetCurrencies>(
                self.oracle.clone(),
                in_lpn,
                self.querier,
            )
        })
        .map_err(DexError::MinOutput)
    }
}
