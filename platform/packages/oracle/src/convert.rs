use std::marker::PhantomData;

use serde::Deserialize;

use currency::{Currency, Group};
use finance::{coin::Coin, price};
#[cfg(feature = "unchecked-base-currency")]
use sdk::cosmwasm_std::Addr;
use sdk::cosmwasm_std::QuerierWrapper;

use crate::{
    error::Error,
    stub::{Oracle, OracleRef, WithOracle},
};

pub fn to_base<BaseC, BaseG, InC, InG>(
    oracle_ref: OracleRef,
    in_amount: Coin<InC>,
    querier: &QuerierWrapper<'_>,
) -> Result<Coin<BaseC>, Error>
where
    BaseC: Currency,
    BaseG: Group + for<'de> Deserialize<'de>,
    InC: Currency,
    InG: Group + for<'de> Deserialize<'de>,
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
        InG: Group + for<'de> Deserialize<'de>,
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

    oracle_ref.execute_as_oracle::<BaseC, BaseG, _>(
        PriceConvert {
            in_amount,
            _in_group: PhantomData::<InG>,
            _out: PhantomData::<BaseC>,
        },
        querier,
    )
}

#[cfg(feature = "unchecked-base-currency")]
pub fn from_unchecked_base<BaseC, BaseG, OutC, OutG>(
    oracle: Addr,
    in_amount: Coin<BaseC>,
    querier: &QuerierWrapper<'_>,
) -> Result<Coin<OutC>, Error>
where
    BaseC: Currency,
    BaseG: Group + for<'de> Deserialize<'de>,
    OutC: Currency,
    OutG: Group + for<'de> Deserialize<'de>,
{
    use crate::stub;

    struct PriceConvert<BaseC, OutC, OutG>
    where
        BaseC: Currency,
        OutC: Currency,
        OutG: Group,
    {
        in_amount: Coin<BaseC>,
        _out: PhantomData<OutC>,
        _out_group: PhantomData<OutG>,
    }

    impl<BaseC, OutC, OutG> WithOracle<BaseC> for PriceConvert<BaseC, OutC, OutG>
    where
        BaseC: Currency,
        OutC: Currency,
        OutG: Group + for<'de> Deserialize<'de>,
    {
        type Output = Coin<OutC>;
        type Error = Error;

        fn exec<OracleImpl>(self, oracle: OracleImpl) -> Result<Self::Output, Self::Error>
        where
            OracleImpl: Oracle<BaseC>,
        {
            Ok(price::total(
                self.in_amount,
                oracle.price_of::<OutC, OutG>()?.inv(),
            ))
        }
    }

    stub::execute_as_unchecked_base_currency_oracle::<BaseC, BaseG, _>(
        oracle,
        PriceConvert {
            in_amount,
            _out: PhantomData::<OutC>,
            _out_group: PhantomData::<OutG>,
        },
        querier,
    )
}
