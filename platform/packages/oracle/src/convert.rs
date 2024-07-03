use currency::{group::MemberOf, Currency, Group};
use finance::coin::Coin;

use crate::{error::Error, stub::Oracle};

use self::impl_::PriceConvert;

pub fn from_quote<QuoteC, OracleS, OutC, OutG>(
    oracle: &OracleS,
    in_amount: Coin<QuoteC>,
) -> Result<Coin<OutC>, Error<OutG>>
where
    QuoteC: Currency,
    OracleS: Oracle<G = OutG, QuoteC = QuoteC, QuoteG = QuoteC::Group>,
    OutC: Currency + MemberOf<OutG>,
    OutG: Group,
{
    PriceConvert::new(in_amount).with_quote_in::<_, OutG>(oracle)
}

pub fn to_quote<InC, InG, QuoteC, OracleS>(
    oracle: &OracleS,
    in_amount: Coin<InC>,
) -> Result<Coin<QuoteC>, Error<InG>>
where
    InC: Currency + MemberOf<InG>,
    InG: Group,
    QuoteC: Currency,
    OracleS: Oracle<G = InG, QuoteC = QuoteC, QuoteG = QuoteC::Group>,
{
    PriceConvert::new(in_amount).with_quote_out::<InG, _>(oracle)
}

mod impl_ {
    use std::marker::PhantomData;

    use currency::{group::MemberOf, Currency, Group};
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
        ) -> Result<Coin<OutC>, Error<OutG>>
        where
            OracleImpl: Oracle<G = OutG, QuoteC = InC, QuoteG = InC::Group>,
            OutC: MemberOf<OutG>,
            OutG: Group,
        {
            oracle
                .price_of::<OutC>()
                .map(|price| price::total(self.in_amount, price.inv()))
        }

        pub(super) fn with_quote_out<InG, OracleImpl>(
            &self,
            oracle: &OracleImpl,
        ) -> Result<Coin<OutC>, Error<InG>>
        where
            InC: MemberOf<InG>,
            InG: Group,
            OracleImpl: Oracle<G = InG, QuoteC = OutC, QuoteG = OutC::Group>,
        {
            oracle
                .price_of::<InC>()
                .map(|price| price::total(self.in_amount, price))
        }
    }
}
