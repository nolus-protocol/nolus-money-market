use std::marker::PhantomData;

use currency::{Currency, CurrencyDef, Group, MemberOf};
use finance::{
    coin::{Coin, CoinDTO, WithCoin, WithCoinResult},
    percent::Percent,
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
    max_slippage: Percent,
    oracle: OracleRef<OutC, OutG>,
    _in_g: PhantomData<InG>,
}

impl<InG, OutC, OutG> MaxSlippage<InG, OutC, OutG>
where
    InG: Group,
    OutC: Currency + MemberOf<OutG>,
    OutG: Group,
{
    pub fn with(max_slippage: Percent, oracle: OracleRef<OutC, OutG>) -> Self {
        Self {
            max_slippage,
            oracle,
            _in_g: PhantomData,
        }
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
            oracle: OracleRef<OutC, OutG>,
            querier: QuerierWrapper<'querier>,
            _in_g: PhantomData<InG>,
        }

        impl<'querier, InG, OutC, OutG> WithCoin<InG> for InCoinResolve<'querier, InG, OutC, OutG>
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
            }
        }

        input.with_coin(InCoinResolve {
            oracle: self.oracle.clone(),
            querier,
            _in_g: PhantomData,
        })
    }
}
