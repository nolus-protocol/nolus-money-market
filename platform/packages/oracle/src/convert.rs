use std::marker::PhantomData;

use serde::Deserialize;

use currency::{Currency, Group};
use finance::{coin::Coin, price};
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

pub fn from_base<BaseC, BaseG, OracleS, OutC, OutG>(
    oracle: &OracleS,
    in_amount: Coin<BaseC>,
) -> Result<Coin<OutC>, Error>
where
    BaseC: Currency,
    BaseG: Group + for<'de> Deserialize<'de>,
    OracleS: Oracle<BaseC>,
    OutC: Currency,
    OutG: Group + for<'de> Deserialize<'de>,
{
    from_base::PriceConvert::<_, _, OutG>::new(in_amount).do_convert(oracle)
}

mod from_base {
    use std::marker::PhantomData;

    use serde::Deserialize;

    use currency::{Currency, Group};
    use finance::{coin::Coin, price};

    use crate::{error::Error, Oracle};

    pub(super) struct PriceConvert<BaseC, OutC, OutG>
    where
        BaseC: Currency,
        OutC: Currency,
        OutG: Group,
    {
        in_amount: Coin<BaseC>,
        _out: PhantomData<OutC>,
        _out_group: PhantomData<OutG>,
    }

    impl<BaseC, OutC, OutG> PriceConvert<BaseC, OutC, OutG>
    where
        BaseC: Currency,
        OutC: Currency,
        OutG: Group + for<'a> Deserialize<'a>,
    {
        pub(super) fn new(in_amount: Coin<BaseC>) -> Self {
            Self {
                in_amount,
                _out: PhantomData,
                _out_group: PhantomData,
            }
        }

        pub(super) fn do_convert<OracleImpl>(
            &self,
            oracle: &OracleImpl,
        ) -> Result<Coin<OutC>, Error>
        where
            OracleImpl: Oracle<BaseC>,
        {
            oracle
                .price_of::<OutC, OutG>()
                .map(|price| price::total(self.in_amount, price.inv()))
        }
    }
}
