use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use currency::{Currency, CurrencyDef, Group, MemberOf};
use finance::{
    coin::{Coin, CoinDTO, WithCoin},
    fraction::Fraction,
    percent::Percent100,
};
use oracle::stub;
use oracle_platform::OracleRef;
use platform::batch::{Emit, Emitter};
use sdk::cosmwasm_std::QuerierWrapper;

use crate::{
    SlippageCalculator,
    error::{Error, Result},
};

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "testing", derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct MaxSlippage(Percent100);

impl MaxSlippage {
    #[cfg(feature = "testing")]
    pub const fn unchecked(max: Percent100) -> Self {
        Self(max)
    }

    pub fn emit<Key>(&self, emitter: Emitter, key: Key) -> Emitter
    where
        Key: Into<String>,
    {
        emitter.emit_percent_amount(key, self.0)
    }

    pub fn min_out<C>(&self, amount_in: Coin<C>) -> Coin<C> {
        self.0.complement().of(amount_in)
    }
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "testing", derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Calculator<InG, OutC, OutG>
where
    InG: Group,
    OutC: Currency + MemberOf<OutG>,
    OutG: Group,
{
    max_slippage: MaxSlippage,
    oracle: OracleRef<OutC, OutG>,
    #[serde(skip)]
    _in_g: PhantomData<InG>,
}

impl<InG, OutC, OutG> Calculator<InG, OutC, OutG>
where
    InG: Group,
    OutC: Currency + MemberOf<OutG>,
    OutG: Group,
{
    pub const fn with(max_slippage: MaxSlippage, oracle: OracleRef<OutC, OutG>) -> Self {
        Self {
            max_slippage,
            oracle,
            _in_g: PhantomData,
        }
    }

    pub const fn threshold(&self) -> MaxSlippage {
        self.max_slippage
    }
}

impl<InG, OutC, OutG> SlippageCalculator<InG> for Calculator<InG, OutC, OutG>
where
    InG: Group,
    OutC: CurrencyDef,
    OutC::Group: MemberOf<OutG> + MemberOf<InG::TopG>,
    OutG: Group,
{
    type OutC = OutC;

    fn min_output(
        &self,
        input: &CoinDTO<InG>,
        querier: QuerierWrapper<'_>,
    ) -> Result<Coin<Self::OutC>> {
        struct InCoinResolve<'querier, InG, OutC, OutG>
        where
            InG: Group,
            OutC: CurrencyDef,
            OutC::Group: MemberOf<OutG> + MemberOf<InG::TopG>,
            OutG: Group,
        {
            max_slippage: MaxSlippage,
            oracle: OracleRef<OutC, OutG>,
            querier: QuerierWrapper<'querier>,
            _in_g: PhantomData<InG>,
        }

        impl<InG, OutC, OutG> WithCoin<InG> for InCoinResolve<'_, InG, OutC, OutG>
        where
            InG: Group,
            OutC: CurrencyDef,
            OutC::Group: MemberOf<OutG> + MemberOf<InG::TopG>,
            OutG: Group,
        {
            type Outcome = Result<Coin<OutC>>;

            fn on<C>(self, input: Coin<C>) -> Self::Outcome
            where
                C: CurrencyDef,
                C::Group: MemberOf<InG> + MemberOf<<InG as Group>::TopG>,
            {
                stub::to_quote::<_, InG, _, _>(self.oracle, input, self.querier)
                    .map_err(Error::MinOutput)
                    .map(|input_in_out_c| self.max_slippage.min_out(input_in_out_c))
            }
        }

        input.with_coin(InCoinResolve {
            max_slippage: self.max_slippage,
            oracle: self.oracle.clone(),
            querier,
            _in_g: PhantomData,
        })
    }
}

#[cfg(test)]
mod test {
    use currency::test::SuperGroupTestC1;
    use finance::{
        coin::{Amount, Coin},
        fraction::Fraction,
        percent::Percent100,
    };

    use super::MaxSlippage;

    #[test]
    fn zero() {
        assert!(calc_min_out(coin(456), Percent100::from_percent(100)).is_zero());
    }

    #[test]
    fn hundred() {
        let coin_in = coin(100);
        assert_eq!(coin_in, calc_min_out(coin_in, Percent100::ZERO));
    }

    #[test]
    fn eighty_five() {
        let coin_in = coin(267);
        let slippage = Percent100::from_percent(15);
        assert_eq!(
            slippage.complement().of(coin_in),
            calc_min_out(coin_in, slippage)
        );
    }

    fn coin(amount: Amount) -> Coin<SuperGroupTestC1> {
        Coin::new(amount)
    }

    fn calc_min_out(
        amount_in: Coin<SuperGroupTestC1>,
        slippage: Percent100,
    ) -> Coin<SuperGroupTestC1> {
        MaxSlippage(slippage).min_out(amount_in)
    }
}
