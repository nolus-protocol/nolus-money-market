use std::marker::PhantomData;

use currency::{Currency, Group};
use finance::coin::Coin;
use sdk::cosmwasm_std::QuerierWrapper;

use oracle_platform::{
    error::{Error, Result},
    Oracle, OracleRef, WithOracle,
};

pub fn from_quote<QuoteC, QuoteG, OutC, OutG>(
    oracle_ref: OracleRef<QuoteC>,
    in_amount: Coin<QuoteC>,
    querier: QuerierWrapper<'_>,
) -> Result<Coin<OutC>>
where
    QuoteC: Currency,
    QuoteG: Group,
    OutC: Currency,
    OutG: Group,
{
    struct PriceConvert<QuoteC, OutC, OutG>
    where
        QuoteC: Currency,
        OutC: Currency,
        OutG: Group,
    {
        in_amount: Coin<QuoteC>,
        _out: PhantomData<OutC>,
        _out_group: PhantomData<OutG>,
    }

    impl<QuoteC, OutC, OutG> WithOracle<QuoteC> for PriceConvert<QuoteC, OutC, OutG>
    where
        QuoteC: Currency,
        OutC: Currency,
        OutG: Group,
    {
        type Output = Coin<OutC>;
        type Error = Error;

        fn exec<OracleImpl>(self, oracle: OracleImpl) -> Result<Self::Output>
        where
            OracleImpl: Oracle<QuoteC>,
        {
            oracle_platform::convert::from_quote::<_, _, _, OutG>(&oracle, self.in_amount)
        }
    }

    oracle_ref.execute_as_oracle::<QuoteG, _>(
        PriceConvert {
            in_amount,
            _out: PhantomData::<OutC>,
            _out_group: PhantomData::<OutG>,
        },
        querier,
    )
}

pub fn to_quote<InC, InG, QuoteC, QuoteG>(
    oracle_ref: OracleRef<QuoteC>,
    in_amount: Coin<InC>,
    querier: QuerierWrapper<'_>,
) -> Result<Coin<QuoteC>>
where
    QuoteC: Currency,
    QuoteG: Group,
    InC: Currency,
    InG: Group,
{
    struct PriceConvert<InC, InG, QuoteC>
    where
        InC: Currency,
        InG: Group,
        QuoteC: Currency,
    {
        in_amount: Coin<InC>,
        _in_group: PhantomData<InG>,
        _out: PhantomData<QuoteC>,
    }

    impl<InC, InG, QuoteC> WithOracle<QuoteC> for PriceConvert<InC, InG, QuoteC>
    where
        InC: Currency,
        InG: Group,
        QuoteC: Currency,
    {
        type Output = Coin<QuoteC>;
        type Error = Error;

        fn exec<OracleImpl>(self, oracle: OracleImpl) -> Result<Self::Output>
        where
            OracleImpl: Oracle<QuoteC>,
        {
            oracle_platform::convert::to_quote::<_, InG, _, _>(&oracle, self.in_amount)
        }
    }

    oracle_ref.execute_as_oracle::<QuoteG, _>(
        PriceConvert {
            in_amount,
            _in_group: PhantomData::<InG>,
            _out: PhantomData::<QuoteC>,
        },
        querier,
    )
}
