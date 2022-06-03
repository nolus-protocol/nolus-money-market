use std::{
    marker::PhantomData,
    ops::{Add, Sub},
};
// , fmt::{Formatter, Result as FmtResult}

use cosmwasm_std::{Coin, Uint128};
use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};

trait Currency {
    const DENOM: &'static str;
}
#[derive(PartialEq, Debug)]
struct Usdc;
impl Currency for Usdc {
    const DENOM: &'static str = "uusdc";
}

#[derive(PartialEq, Debug)]
struct Nls;
impl Currency for Nls {
    const DENOM: &'static str = "unls";
}

#[derive(Deserialize, PartialEq, Clone, Copy, Debug)]
pub struct MyCoin<C> {
    amount: Uint128,
    denom: PhantomData<C>,
}

impl<C> MyCoin<C> {
    pub fn new(amount: u128) -> Self {
        Self {
            amount: amount.into(),
            denom: PhantomData::<C>,
        }
    }
}
impl<C> Add<MyCoin<C>> for MyCoin<C> {
    type Output = Self;

    fn add(self, rhs: MyCoin<C>) -> Self::Output {
        Self::Output {
            amount: self.amount + rhs.amount,
            denom: self.denom,
        }
    }
}

impl<C> Sub<MyCoin<C>> for MyCoin<C> {
    type Output = Self;

    fn sub(self, rhs: MyCoin<C>) -> Self::Output {
        Self::Output {
            amount: self.amount - rhs.amount,
            denom: self.denom,
        }
    }
}

impl<C> Serialize for MyCoin<C>
where
    C: Currency,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut rgb = serializer.serialize_struct("MyCoin", 2)?;
        rgb.serialize_field("amount", &self.amount)?;

        rgb.serialize_field("denom", &C::DENOM)?;

        rgb.end()
    }
}

// impl<'de, C> Deserialize<'de> for MyCoin<C> {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         deserializer.deserialize_str(DecimalVisitor)
//     }
// }

// struct DecimalVisitor;

// impl<'de, C> Visitor<'de> for DecimalVisitor {
//     type Value = MyCoin<C>;

//     fn expecting(&self, formatter: &mut Formatter) -> FmtResult {
//         formatter.write_str("string-encoded decimal")
//     }

//     fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
//     where
//         E: Error,
//     {
//         match Decimal::from_str(v) {
//             Ok(d) => Ok(d),
//             Err(e) => Err(E::custom(format!("Error parsing decimal '{}': {}", v, e))),
//         }
//     }
// }

pub fn sub_amount(from: Coin, amount: Uint128) -> Coin {
    Coin {
        amount: from.amount - amount,
        denom: from.denom,
    }
}

pub fn add_coin(to: Coin, other: Coin) -> Coin {
    debug_assert!(to.denom == other.denom);
    Coin {
        amount: to.amount + other.amount,
        denom: to.denom,
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::to_vec;

    use crate::coin::Usdc;

    use super::{MyCoin, Nls};

    #[test]
    fn serialize() {
        let amount = 123;
        let coin_nls = MyCoin::<Nls>::new(amount);
        let coin_usdc = MyCoin::<Usdc>::new(amount);

        let coin_usdc_bin = to_vec(&coin_nls).unwrap();

        let coin_nls_txt = String::from_utf8(coin_usdc_bin).unwrap();
        let coin_usdc_txt = String::from_utf8(to_vec(&coin_usdc).unwrap()).unwrap();
        assert_ne!(coin_nls_txt, coin_usdc_txt);
    
        // let coin_usdc_deser: MyCoin<Usdc> = from_slice(&coin_usdc_bin).unwrap();
        // assert_eq!(coin_usdc_deser, coin_usdc);
        assert_eq!(r#"{"amount":"123","denom":"uusdc"}"#, coin_usdc_txt);
        assert_eq!(r#"{"amount":"123","denom":"unls"}"#, coin_nls_txt);
    }
}
