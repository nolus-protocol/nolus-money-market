use currency::{Currency, Group};
use finance::coin::Coin;

use crate::{error::Error, stub::Oracle};

use self::impl_::PriceConvert;

pub fn from_quote<QuoteC, OracleS, OutC, OutG>(
    oracle: &OracleS,
    in_amount: Coin<QuoteC>,
) -> Result<Coin<OutC>, Error>
where
    QuoteC: Currency,
    OracleS: Oracle<QuoteC>,
    OutC: Currency,
    OutG: Group,
{
    PriceConvert::new(in_amount).with_quote_in::<_, OutG>(oracle)
}

pub fn to_quote<InC, InG, QuoteC, OracleS>(
    oracle: &OracleS,
    in_amount: Coin<InC>,
) -> Result<Coin<QuoteC>, Error>
where
    InC: Currency,
    InG: Group,
    QuoteC: Currency,
    OracleS: Oracle<QuoteC>,
{
    PriceConvert::new(in_amount).with_quote_out::<InG, _>(oracle)
}

mod impl_ {
    use std::marker::PhantomData;

    use currency::{Currency, Group};
    use finance::{coin::Coin, price};

    use crate::{error::Error, Oracle};

    pub(super) struct PriceConvert<InC, OutC>
    where
        InC: Currency,
        OutC: Currency,
    {
        in_amount: Coin<InC>,
        _out: PhantomData<OutC>,
    }

    impl<InC, OutC> PriceConvert<InC, OutC>
    where
        InC: Currency,
        OutC: Currency,
    {
        pub(super) fn new(in_amount: Coin<InC>) -> Self {
            Self {
                in_amount,
                _out: PhantomData,
            }
        }

        pub(super) fn with_quote_in<OracleImpl, OutG>(
            &self,
            oracle: &OracleImpl,
        ) -> Result<Coin<OutC>, Error>
        where
            OracleImpl: Oracle<InC>,
            OutG: Group,
        {
            oracle
                .price_of::<OutC, OutG>()
                .map(|price| price::total(self.in_amount, price.inv()))
        }

        pub(super) fn with_quote_out<InG, OracleImpl>(
            &self,
            oracle: &OracleImpl,
        ) -> Result<Coin<OutC>, Error>
        where
            InG: Group,
            OracleImpl: Oracle<OutC>,
        {
            oracle
                .price_of::<InC, InG>()
                .map(|price| price::total(self.in_amount, price))
        }
    }
}
