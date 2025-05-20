use std::marker::PhantomData;

use currency::{Currency, CurrencyDef, Group, MemberOf};
use finance::{
    coin::{Coin, CoinDTO, WithCoin, WithCoinResult},
    fraction::Fraction,
    percent::{Percent, bound::BoundToHundredPercent},
};
use oracle::stub;
use oracle_platform::OracleRef;
use sdk::cosmwasm_std::QuerierWrapper;
use serde::{Deserialize, Serialize};

use crate::{
    SlippageCalculator,
    error::{Error, Result},
};

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "testing", derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct MaxSlippage<InG, OutC, OutG>
where
    InG: Group,
    OutC: Currency + MemberOf<OutG>,
    OutG: Group,
{
    max_slippage: BoundToHundredPercent,
    oracle: OracleRef<OutC, OutG>,
    _in_g: PhantomData<InG>,
}

impl<InG, OutC, OutG> MaxSlippage<InG, OutC, OutG>
where
    InG: Group,
    OutC: Currency + MemberOf<OutG>,
    OutG: Group,
{
    pub fn with(max_slippage: BoundToHundredPercent, oracle: OracleRef<OutC, OutG>) -> Self {
        Self {
            max_slippage,
            oracle,
            _in_g: PhantomData,
        }
    }

    pub fn threshold(&self) -> BoundToHundredPercent {
        self.max_slippage
    }
}

impl<InG, OutC, OutG> SlippageCalculator<InG> for MaxSlippage<InG, OutC, OutG>
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
            max_slippage: BoundToHundredPercent,
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
            type Output = Coin<OutC>;

            type Error = Error;

            fn on<C>(self, input: Coin<C>) -> WithCoinResult<InG, Self>
            where
                C: CurrencyDef,
                C::Group: MemberOf<InG> + MemberOf<<InG as Group>::TopG>,
            {
                stub::to_quote::<_, InG, _, _>(self.oracle, input, self.querier)
                    .map_err(Self::Error::MinOutput)
                    .map(|input_in_out_c| calc_min_out(input_in_out_c, self.max_slippage))
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

fn calc_min_out<C>(amount: Coin<C>, slippage: BoundToHundredPercent) -> Coin<C> {
    (Percent::HUNDRED - slippage.percent()).of(amount)
}

#[cfg(test)]
mod test {
    use currency::test::SuperGroupTestC1;
    use finance::{
        coin::Coin,
        fraction::Fraction,
        percent::{Percent, bound::BoundToHundredPercent},
    };

    use crate::impl_::slippage::percent::calc_min_out;

    #[test]
    fn zero() {
        assert!(
            calc_min_out(
                Coin::<SuperGroupTestC1>::from(100),
                BoundToHundredPercent::strict_from_percent(Percent::from_percent(100))
            )
            .is_zero()
        );
    }

    #[test]
    fn hundred() {
        let coin_in = Coin::<SuperGroupTestC1>::from(100);
        assert_eq!(
            coin_in,
            calc_min_out(
                coin_in,
                BoundToHundredPercent::strict_from_percent(Percent::ZERO)
            )
        );
    }

    #[test]
    fn eighty_five() {
        let coin_in = Coin::<SuperGroupTestC1>::from(100);
        assert_eq!(
            Percent::from_percent(85).of(coin_in),
            calc_min_out(
                coin_in,
                BoundToHundredPercent::strict_from_percent(Percent::from_percent(15))
            )
        );
    }
}
