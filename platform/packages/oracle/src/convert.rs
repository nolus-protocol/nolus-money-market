use currency::{Currency, Group};
use finance::coin::Coin;

use crate::{error::Error, stub::Oracle};

pub fn from_stable<StableC, StableG, OracleS, OutC, OutG>(
    oracle: &OracleS,
    in_amount: Coin<StableC>,
) -> Result<Coin<OutC>, Error>
where
    StableC: Currency,
    StableG: Group,
    OracleS: Oracle<StableC>,
    OutC: Currency,
    OutG: Group,
{
    from_stable::PriceConvert::<_, _, OutG>::new(in_amount).do_convert(oracle)
}

mod from_stable {
    use std::marker::PhantomData;

    use currency::{Currency, Group};
    use finance::{coin::Coin, price};

    use crate::{error::Error, Oracle};

    pub(super) struct PriceConvert<InC, OutC, OutG>
    where
        InC: Currency,
        OutC: Currency,
        OutG: Group,
    {
        in_amount: Coin<InC>,
        _out: PhantomData<OutC>,
        _out_group: PhantomData<OutG>,
    }

    impl<InC, OutC, OutG> PriceConvert<InC, OutC, OutG>
    where
        InC: Currency,
        OutC: Currency,
        OutG: Group,
    {
        pub(super) fn new(in_amount: Coin<InC>) -> Self {
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
            OracleImpl: Oracle<InC>,
        {
            oracle
                .price_of::<OutC, OutG>()
                .map(|price| price::total(self.in_amount, price.inv()))
        }
    }
}
