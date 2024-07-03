use currency::{Currency, Group};
use finance::coin::Coin;

use crate::{error::Error, stub::Oracle};

use self::impl_::PriceConvert;

pub fn from_quote<QuoteC, QuoteG, OracleS, OutC, OutG>(
    oracle: &OracleS,
    in_amount: Coin<QuoteC>,
) -> Result<Coin<OutC>, Error>
where
    QuoteC: Currency,
    QuoteG: Group,
    OracleS: Oracle<QuoteC, QuoteG>,
    OutC: Currency,
    OutG: Group,
{
    PriceConvert::<QuoteC, QuoteG, OutC, OutG>::new(in_amount).with_quote_in(oracle)
}

pub fn to_quote<InC, InG, QuoteC, QuoteG, OracleS>(
    oracle: &OracleS,
    in_amount: Coin<InC>,
) -> Result<Coin<QuoteC>, Error>
where
    InC: Currency,
    InG: Group,
    QuoteC: Currency,
    QuoteG: Group,
    OracleS: Oracle<QuoteC, QuoteG>,
{
    PriceConvert::<InC, InG, QuoteC, QuoteG>::new(in_amount).with_quote_out(oracle)
}

mod impl_ {
    use std::marker::PhantomData;

    use currency::{Currency, Group};
    use finance::{coin::Coin, price};

    use crate::{error::Error, Oracle};

    pub(super) struct PriceConvert<InC, InG, OutC, OutG>
    where
        InC: Currency,
        InG: Group,
        OutC: Currency,
        OutG: Group,
    {
        in_amount: Coin<InC>,
        _in_group: PhantomData<InG>,
        _out: PhantomData<OutC>,
        _out_group: PhantomData<OutG>,
    }

    impl<InC, InG, OutC, OutG> PriceConvert<InC, InG, OutC, OutG>
    where
        InC: Currency,
        InG: Group,
        OutC: Currency,
        OutG: Group,
    {
        pub(super) fn new(in_amount: Coin<InC>) -> Self {
            Self {
                in_amount,
                _in_group: PhantomData,
                _out: PhantomData,
                _out_group: PhantomData,
            }
        }

        pub(super) fn with_quote_in<OracleImpl>(
            &self,
            oracle: &OracleImpl,
        ) -> Result<Coin<OutC>, Error>
        where
            OracleImpl: Oracle<InC, InG>,
        {
            oracle
                .price_of::<OutC, OutG>()
                .map(|price| price::total(self.in_amount, price.inv()))
        }

        pub(super) fn with_quote_out<OracleImpl>(
            &self,
            oracle: &OracleImpl,
        ) -> Result<Coin<OutC>, Error>
        where
            OracleImpl: Oracle<OutC, OutG>,
        {
            oracle
                .price_of::<InC, InG>()
                .map(|price| price::total(self.in_amount, price))
        }
    }
}
