use std::marker::PhantomData;

use currency::{Currency, Group};
use finance::coin::Coin;
use sdk::cosmwasm_std::QuerierWrapper;

use super::{error::Error, Oracle, OracleRef, WithOracle};

pub fn to_quote<InC, InG, QuoteC, QuoteG>(
    oracle_ref: OracleRef<QuoteC>,
    in_amount: Coin<InC>,
    querier: QuerierWrapper<'_>,
) -> Result<Coin<QuoteC>, Error>
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

        fn exec<OracleImpl>(self, oracle: OracleImpl) -> Result<Self::Output, Self::Error>
        where
            OracleImpl: Oracle<QuoteC>,
        {
            oracle_platform::convert::to_quote(oracle, self.in_amount)
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
