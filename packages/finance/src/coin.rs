use std::{
    fmt::{Debug, Display, Formatter, Write},
    marker::PhantomData,
    ops::{Add, Sub},
};

use schemars::JsonSchema;
use serde::{
    de::{Error, SeqAccess, Unexpected, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};

use crate::currency::Currency;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, JsonSchema)]
pub struct Coin<C> {
    amount: u128,
    currency: PhantomData<C>,
}

impl<C> Coin<C> {
    pub fn new(amount: u128) -> Self {
        Self {
            amount,
            currency: PhantomData::<C>,
        }
    }

    pub(super) fn amount(&self) -> u128 {
        self.amount
    }
}
impl<C> Add<Coin<C>> for Coin<C> {
    type Output = Self;

    fn add(self, rhs: Coin<C>) -> Self::Output {
        Self::Output {
            amount: self.amount + rhs.amount,
            currency: self.currency,
        }
    }
}

impl<C> Sub<Coin<C>> for Coin<C> {
    type Output = Self;

    fn sub(self, rhs: Coin<C>) -> Self::Output {
        Self::Output {
            amount: self.amount - rhs.amount,
            currency: self.currency,
        }
    }
}

impl<C> Serialize for Coin<C>
where
    C: Currency,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeTuple;

        let mut rgb = serializer.serialize_tuple(2)?;
        rgb.serialize_element(&self.amount)?;
        rgb.serialize_element(&C::SYMBOL)?;
        rgb.end()
    }
}

impl<'de, C> Deserialize<'de> for Coin<C>
where
    C: Currency,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_tuple(2, CoinVisitor::<C>(PhantomData))
    }
}

struct CoinVisitor<C>(PhantomData<C>);

impl<'de, C> Visitor<'de> for CoinVisitor<C>
where
    C: Currency,
{
    type Value = Coin<C>;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("a Coin encoded in two fields, amount and currency label")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let amount = seq
            .next_element()?
            .ok_or_else(|| Error::invalid_length(0, &self))?;
        let label: &str = seq
            .next_element()?
            .ok_or_else(|| Error::invalid_length(1, &self))?;
        if label != C::SYMBOL {
            Err(Error::invalid_value(Unexpected::Str(label), &C::SYMBOL))
        } else {
            Ok(Coin::<C>::new(amount))
        }
    }
}

impl<C> Display for Coin<C>
where
    C: Currency,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.amount().to_string())?;
        f.write_char(' ')?;
        f.write_str(C::SYMBOL)?;
        Ok(())
    }
}

impl<C> From<u128> for Coin<C> {
    fn from(amount: u128) -> Self {
        Self::new(amount)
    }
}

impl<C> From<Coin<C>> for u128 {
    fn from(coin: Coin<C>) -> Self {
        coin.amount()
    }
}

#[cfg(test)]
mod test {
    use std::{any::type_name, fmt::Debug};

    use cosmwasm_std::{from_slice, to_vec, StdError};
    use serde::Serialize;

    use crate::{
        currency::{Currency, Nls, Usdc},
        percent::test::{test_are, test_of, test_of_are},
    };

    use super::Coin;

    #[test]
    fn serialize_deserialize() {
        serialize_deserialize_impl::<Nls>(u128::MIN, r#"["0","unls"]"#);
        serialize_deserialize_impl::<Nls>(123, r#"["123","unls"]"#);
        serialize_deserialize_impl::<Nls>(
            u128::MAX,
            r#"["340282366920938463463374607431768211455","unls"]"#,
        );
        serialize_deserialize_impl::<Usdc>(u128::MIN, r#"["0","uusdc"]"#);
        serialize_deserialize_impl::<Usdc>(7368953, r#"["7368953","uusdc"]"#);
        serialize_deserialize_impl::<Usdc>(
            u128::MAX,
            r#"["340282366920938463463374607431768211455","uusdc"]"#,
        );
    }

    fn serialize_deserialize_impl<C>(amount: u128, exp_txt: &str)
    where
        C: Currency + PartialEq + Debug,
    {
        let coin = Coin::<C>::new(amount);
        let coin_bin = to_vec(&coin).unwrap();
        assert_eq!(coin, from_slice(&coin_bin).unwrap());

        let coin_txt = String::from_utf8(coin_bin).unwrap();
        assert_eq!(exp_txt, coin_txt);

        assert_eq!(coin, from_slice(exp_txt.as_bytes()).unwrap());
    }
    #[test]
    fn serialize_as_field() {
        //tested since it requires Serialize of the Currency
        #[derive(Serialize)]
        struct CoinContainer<C>
        where
            C: Currency,
            Coin<C>: Serialize,
        {
            coin: Coin<C>,
        }
        assert_eq!(
            r#"{"coin":["10","uusdc"]}"#,
            String::from_utf8(
                to_vec(&CoinContainer { coin: usdc(10) }).expect("serialization failed")
            )
            .unwrap()
        );
    }

    #[test]
    fn distinct_repr() {
        let amount = 432;
        assert_ne!(
            to_vec(&nls(amount)).unwrap(),
            to_vec(&usdc(amount)).unwrap()
        );
    }

    #[test]
    fn wrong_currency() {
        let amount = 134;
        let nls_bin = to_vec(&nls(amount)).unwrap();
        let res = from_slice::<Coin<Usdc>>(&nls_bin);
        assert_eq!(
            Err(StdError::parse_err(
                type_name::<Coin::<Usdc>>(),
                "invalid value: string \"unls\", expected uusdc"
            )),
            res
        );
    }

    #[test]
    fn display() {
        assert_eq!("25 unls", nls(25).to_string());
        assert_eq!("0 uusdc", usdc(0).to_string());
    }

    #[test]
    fn of_are() {
        test_of_are(10, usdc(100), usdc(1));
        test_of(11, usdc(100), usdc(1));
        test_are(11, usdc(1), usdc(90));
        test_of(110, usdc(100), usdc(11));
        test_are(110, usdc(11), usdc(100));
        test_of(12, usdc(100), usdc(1));
        test_are(12, usdc(1), usdc(83));
        test_of(18, usdc(100), usdc(1));
        test_are(18, usdc(1), usdc(55));
        test_of(18, usdc(120), usdc(2));
        test_are(18, usdc(2), usdc(111));
        test_of_are(1000, usdc(u128::MAX), usdc(u128::MAX));
    }

    #[test]
    #[should_panic]
    fn of_overflow() {
        let max_amount = usdc(u128::MAX);
        test_of(1001, max_amount, max_amount);
    }
    #[test]
    #[should_panic]
    fn are_overflow() {
        let max_amount = usdc(u128::MAX);
        test_are(999, max_amount, max_amount);
    }

    fn usdc(amount: u128) -> Coin<Usdc> {
        Coin::new(amount)
    }

    fn nls(amount: u128) -> Coin<Nls> {
        Coin::new(amount)
    }
}
