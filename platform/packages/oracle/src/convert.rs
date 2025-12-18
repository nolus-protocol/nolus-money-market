use currency::{CurrencyDef, Group, MemberOf};
use finance::coin::Coin;

use crate::{error::Error, stub::Oracle};

use self::impl_::PriceConvert;

pub fn from_quote<QuoteC, QuoteG, OracleS, OutC, OutG>(
    oracle: &OracleS,
    in_amount: Coin<QuoteC>,
) -> Result<Coin<OutC>, Error>
where
    QuoteC: CurrencyDef,
    QuoteC::Group: MemberOf<QuoteG>,
    QuoteG: Group,
    OracleS: Oracle<OutG, QuoteC = QuoteC, QuoteG = QuoteG>,
    OutC: CurrencyDef,
    OutC::Group: MemberOf<OutG>,
    OutG: Group,
{
    PriceConvert::<QuoteC, QuoteG, OutC, OutG>::new(in_amount).with_quote_in(oracle)
}

pub fn to_quote<InC, InG, QuoteC, QuoteG, OracleS>(
    oracle: &OracleS,
    in_amount: Coin<InC>,
) -> Result<Coin<QuoteC>, Error>
where
    InC: CurrencyDef,
    InC::Group: MemberOf<InG>,
    InG: Group,
    QuoteC: CurrencyDef,
    QuoteC::Group: MemberOf<QuoteG>,
    QuoteG: Group,
    OracleS: Oracle<InG, QuoteC = QuoteC, QuoteG = QuoteG>,
{
    PriceConvert::<InC, InG, QuoteC, QuoteG>::new(in_amount).with_quote_out(oracle)
}

mod impl_ {
    use std::marker::PhantomData;

    use currency::{Currency, CurrencyDef, Group, MemberOf};
    use finance::{
        coin::Coin,
        price::{self, Price},
    };

    use crate::{Oracle, error::Error};

    pub(super) struct PriceConvert<InC, InG, OutC, OutG>
    where
        InC: Currency + MemberOf<InG>,
        InG: Group,
        OutC: Currency + MemberOf<OutG>,
        OutG: Group,
    {
        in_amount: Coin<InC>,
        _in_group: PhantomData<InG>,
        _out: PhantomData<OutC>,
        _out_group: PhantomData<OutG>,
    }

    impl<InC, InG, OutC, OutG> PriceConvert<InC, InG, OutC, OutG>
    where
        InC: CurrencyDef,
        InC::Group: MemberOf<InG>,
        InG: Group,
        OutC: CurrencyDef,
        OutC::Group: MemberOf<OutG>,
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
            OracleImpl: Oracle<OutG, QuoteC = InC, QuoteG = InG>,
        {
            oracle
                .price_of::<OutC>()
                .and_then(|price| self.total_with(price.inv()))
        }

        pub(super) fn with_quote_out<OracleImpl>(
            &self,
            oracle: &OracleImpl,
        ) -> Result<Coin<OutC>, Error>
        where
            OracleImpl: Oracle<InG, QuoteC = OutC, QuoteG = OutG>,
        {
            oracle
                .price_of::<InC>()
                .and_then(|price| self.total_with(price))
        }

        fn total_with(&self, price: Price<InC, OutC>) -> Result<Coin<OutC>, Error> {
            price::total(self.in_amount, price).ok_or(Error::overflow(
                "Overflow while calculating the total value",
            ))
        }
    }
}

#[cfg(test)]
mod test {
    use currency::{platform::Stable, test::SuperGroupTestC1};
    use finance::coin::{Amount, Coin};

    use crate::test::DummyOracle;

    #[test]
    fn from_quote() {
        assert_from_quote(3, 12, 4);
        assert_from_quote(1, 4, 4);
        assert_from_quote(4, 4, 1);
        assert_from_quote(2, 14, 7);
        assert_from_quote(10, 9, 0);
        assert_from_quote(2, Amount::MAX, Amount::MAX / 2);
        assert_from_quote(Amount::MAX / 5, 4, 20 / Amount::MAX);
        assert_from_quote(Amount::MAX, 5, 0);
        assert_from_quote(Amount::MAX, Amount::MAX, 1);
    }

    #[test]
    fn to_quote() {
        assert_to_quote(4, 3, 12);
        assert_to_quote(1, 6, 6);
        assert_to_quote(10, 4, 40);
        assert_to_quote(7, 1, 7);
        assert_to_quote(Amount::MAX / 10, 5, Amount::MAX / 2 - 2);
    }

    #[test]
    fn to_quote_error() {
        let oracle_1 = DummyOracle::with_price(Amount::MAX / 4);
        assert!(super::to_quote(&oracle_1, Coin::<SuperGroupTestC1>::new(8)).is_err());

        let oracle_2 = DummyOracle::with_price(2);
        assert!(
            super::to_quote(
                &oracle_2,
                Coin::<SuperGroupTestC1>::new(Amount::MAX / 2 + 1),
            )
            .is_err()
        );
    }

    fn assert_from_quote(oracle_price: Amount, in_amount: Amount, expected_out: Amount) {
        let oracle = DummyOracle::with_price(oracle_price);

        let out_amount = super::from_quote(&oracle, Coin::new(in_amount)).unwrap();

        assert_eq!(Coin::<SuperGroupTestC1>::new(expected_out), out_amount);
    }

    fn assert_to_quote(oracle_price: Amount, in_amount: Amount, expected_out: Amount) {
        let oracle = DummyOracle::with_price(oracle_price);
        let out_amount =
            super::to_quote(&oracle, Coin::<SuperGroupTestC1>::new(in_amount)).unwrap();
        assert_eq!(Coin::<Stable>::new(expected_out), out_amount);
    }
}
