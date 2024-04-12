use std::marker::PhantomData;

use currency::{Currency, Group};
use finance::{coin::Coin, price};
use sdk::cosmwasm_std::QuerierWrapper;

use super::{error::Error, Oracle, OracleRef, WithOracle};

pub fn to_base<BaseC, BaseG, InC, InG>(
    oracle_ref: OracleRef<BaseC>,
    in_amount: Coin<InC>,
    querier: QuerierWrapper<'_>,
) -> Result<Coin<BaseC>, Error>
where
    BaseC: Currency,
    BaseG: Group,
    InC: Currency,
    InG: Group,
{
    struct PriceConvert<BaseC, InC, InG>
    where
        BaseC: Currency,
        InC: Currency,
        InG: Group,
    {
        in_amount: Coin<InC>,
        _in_group: PhantomData<InG>,
        _out: PhantomData<BaseC>,
    }

    impl<BaseC, InC, InG> WithOracle<BaseC> for PriceConvert<BaseC, InC, InG>
    where
        BaseC: Currency,
        InC: Currency,
        InG: Group,
    {
        type Output = Coin<BaseC>;
        type Error = Error;

        fn exec<OracleImpl>(self, oracle: OracleImpl) -> Result<Self::Output, Self::Error>
        where
            OracleImpl: Oracle<BaseC>,
        {
            oracle
                .price_of::<InC, InG>()
                .map(|price| price::total(self.in_amount, price))
        }
    }

    oracle_ref.execute_as_oracle::<BaseG, _>(
        PriceConvert {
            in_amount,
            _in_group: PhantomData::<InG>,
            _out: PhantomData::<BaseC>,
        },
        querier,
    )
}
