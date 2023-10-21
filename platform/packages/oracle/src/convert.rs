use std::marker::PhantomData;

use serde::Deserialize;

use currency::{Currency, Group};
use finance::{coin::Coin, price};
use sdk::cosmwasm_std::QuerierWrapper;

use crate::{
    error::Error,
    stub::{Oracle, OracleRef, WithOracle},
};

pub fn to_base<BaseC, InC, G>(
    oracle_ref: OracleRef,
    in_amount: Coin<InC>,
    querier: &QuerierWrapper<'_>,
) -> Result<Coin<BaseC>, Error>
where
    BaseC: Currency,
    InC: Currency,
    G: Group + for<'de> Deserialize<'de>,
{
    struct PriceConvert<BaseC, In>
    where
        BaseC: Currency,
        In: Currency,
    {
        in_amount: Coin<In>,
        _out: PhantomData<BaseC>,
    }

    impl<BaseC, In> WithOracle<BaseC> for PriceConvert<BaseC, In>
    where
        BaseC: Currency,
        In: Currency,
    {
        type Output = Coin<BaseC>;
        type Error = Error;

        fn exec<OracleImpl>(self, oracle: OracleImpl) -> Result<Self::Output, Self::Error>
        where
            OracleImpl: Oracle<BaseC>,
        {
            oracle
                .price_of()
                .map(|price| price::total(self.in_amount, price))
        }
    }

    oracle_ref.execute_as_oracle::<_, G, _>(
        PriceConvert {
            in_amount,
            _out: PhantomData,
        },
        querier,
    )
}

pub fn from_base<BaseC, OutC, G>(
    oracle_ref: OracleRef,
    in_amount: Coin<BaseC>,
    querier: &QuerierWrapper<'_>,
) -> Result<Coin<OutC>, Error>
where
    BaseC: Currency,
    OutC: Currency,
    G: Group + for<'de> Deserialize<'de>,
{
    struct PriceConvert<BaseC, Out>
    where
        BaseC: Currency,
        Out: Currency,
    {
        in_amount: Coin<BaseC>,
        _out: PhantomData<Out>,
    }

    impl<BaseC, Out> WithOracle<BaseC> for PriceConvert<BaseC, Out>
    where
        BaseC: Currency,
        Out: Currency,
    {
        type Output = Coin<Out>;
        type Error = Error;

        fn exec<OracleImpl>(self, oracle: OracleImpl) -> Result<Self::Output, Self::Error>
        where
            OracleImpl: Oracle<BaseC>,
        {
            Ok(price::total(self.in_amount, oracle.price_of()?.inv()))
        }
    }

    oracle_ref.execute_as_oracle::<_, G, _>(
        PriceConvert {
            in_amount,
            _out: PhantomData,
        },
        querier,
    )
}
