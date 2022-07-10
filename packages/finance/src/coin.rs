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

    pub fn is_zero(&self) -> bool {
        self.amount == u128::default()
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
    use serde::{de::DeserializeOwned, Deserialize, Serialize};

    use crate::{
        currency::{Currency, Nls, Usdc},
        percent::test::test_of,
    };

    use super::Coin;

    #[test]
    fn serialize_deserialize() {
        serialize_deserialize_coin::<Nls>(u128::MIN, r#"["0","unls"]"#);
        serialize_deserialize_coin::<Nls>(123, r#"["123","unls"]"#);
        serialize_deserialize_coin::<Nls>(
            u128::MAX,
            r#"["340282366920938463463374607431768211455","unls"]"#,
        );
        serialize_deserialize_coin::<Usdc>(u128::MIN, r#"["0","uusdc"]"#);
        serialize_deserialize_coin::<Usdc>(7368953, r#"["7368953","uusdc"]"#);
        serialize_deserialize_coin::<Usdc>(
            u128::MAX,
            r#"["340282366920938463463374607431768211455","uusdc"]"#,
        );
    }

    fn serialize_deserialize_coin<C>(amount: u128, exp_txt: &str)
    where
        C: Currency + PartialEq + Debug,
    {
        let coin = Coin::<C>::new(amount);
        serialize_deserialize_impl(coin, exp_txt)
    }

    fn serialize_deserialize_impl<T>(obj: T, exp_txt: &str)
    where
        T: Serialize + DeserializeOwned + PartialEq + Debug,
    {
        let obj_bin = to_vec(&obj).unwrap();
        assert_eq!(obj, from_slice(&obj_bin).unwrap());

        let obj_txt = String::from_utf8(obj_bin).unwrap();
        assert_eq!(exp_txt, obj_txt);

        assert_eq!(obj, from_slice(exp_txt.as_bytes()).unwrap());
    }
    #[test]
    fn serialize_deserialize_as_field() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct CoinContainer<C>
        where
            C: Currency,
            Coin<C>: Serialize,
        {
            coin: Coin<C>,
        }
        let coin_container = CoinContainer { coin: usdc(10) };
        serialize_deserialize_impl(coin_container, r#"{"coin":["10","uusdc"]}"#);
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
        test_of(10, usdc(100), usdc(1));
        test_of(11, usdc(100), usdc(1));
        test_of(11, usdc(90), usdc(0));
        test_of(11, usdc(91), usdc(1));
        test_of(110, usdc(100), usdc(11));
        test_of(12, usdc(100), usdc(1));
        test_of(12, usdc(84), usdc(1));
        test_of(12, usdc(83), usdc(0));
        test_of(18, usdc(100), usdc(1));
        test_of(18, usdc(56), usdc(1));
        test_of(18, usdc(55), usdc(0));
        test_of(18, usdc(120), usdc(2));
        test_of(18, usdc(112), usdc(2));
        test_of(18, usdc(111), usdc(1));
        test_of(1000, usdc(u128::MAX), usdc(u128::MAX));
    }

    #[test]
    fn is_zero() {
        assert!(usdc(0).is_zero());
        assert!(!usdc(1).is_zero());
    }

    #[test]
    #[should_panic]
    fn of_overflow() {
        let max_amount = usdc(u128::MAX);
        test_of(1001, max_amount, max_amount);
    }
    fn usdc(amount: u128) -> Coin<Usdc> {
        Coin::new(amount)
    }

    fn nls(amount: u128) -> Coin<Nls> {
        Coin::new(amount)
    }
}
