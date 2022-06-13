use std::{
    marker::PhantomData,
    ops::{Add, Sub}, fmt::Debug,
};

use serde::{Deserialize, Serialize};

pub trait Currency : 'static {
    const DENOM: &'static str;
}
#[derive(PartialEq, Debug)]
pub struct Usdc;
impl Currency for Usdc {
    const DENOM: &'static str = "uusdc";
}

#[derive(PartialEq, Debug)]
pub struct Nls;
impl Currency for Nls {
    const DENOM: &'static str = "unls";
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
pub struct Coin<C> {
    amount: u128,
    denom: PhantomData<C>,
}

impl<C> Coin<C> {
    pub fn new(amount: u128) -> Self {
        Self {
            amount,
            denom: PhantomData::<C>,
        }
    }
}
impl<C> Add<Coin<C>> for Coin<C> {
    type Output = Self;

    fn add(self, rhs: Coin<C>) -> Self::Output {
        Self::Output {
            amount: self.amount + rhs.amount,
            denom: self.denom,
        }
    }
}

impl<C> Sub<Coin<C>> for Coin<C> {
    type Output = Self;

    fn sub(self, rhs: Coin<C>) -> Self::Output {
        Self::Output {
            amount: self.amount - rhs.amount,
            denom: self.denom,
        }
    }
}

// impl<C> Serialize for MyCoin<C>
// where
//     C: Currency,
// {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         let mut rgb = serializer.serialize_struct("MyCoin", 2)?;
//         rgb.serialize_field("amount", &self.amount)?;

//         rgb.serialize_field("denom", &C::DENOM)?;

//         rgb.end()
//     }
// }

// fn from<C>(c: Coin) -> Option<MyCoin<C>>
// where
//     C: Currency,
// {
//     if C::DENOM == c.denom.as_str() {
//         Some(MyCoin::new(c.amount.into()))
//     } else {
//         None
//     }
// }

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

#[cfg(test)]
mod test {

    // #[test]
    // fn serialize() {
    //     let amount = 123;
    //     let coin_nls = Coin::<Nls>::new(amount);
    //     let coin_usdc = Coin::<Usdc>::new(amount);

    //     let coin_usdc_bin = to_vec(&coin_nls).unwrap();

    //     let coin_nls_txt = String::from_utf8(coin_usdc_bin).unwrap();
    //     let coin_usdc_txt = String::from_utf8(to_vec(&coin_usdc).unwrap()).unwrap();
    //     assert_ne!(coin_nls_txt, coin_usdc_txt);

    //     // let coin_usdc_deser: MyCoin<Usdc> = from_slice(&coin_usdc_bin).unwrap();
    //     // assert_eq!(coin_usdc_deser, coin_usdc);
    //     assert_eq!(r#"{"amount":"123","denom":"uusdc"}"#, coin_usdc_txt);
    //     assert_eq!(r#"{"amount":"123","denom":"unls"}"#, coin_nls_txt);
    // }

    // #[test]
    // fn from_coin() {
    //     assert!(from::<Usdc>(Coin::new(123, "uuu")).is_none());
    // }

    // #[test]
    // fn test_from_label() {
    //     let c: &'static dyn Currency = &Usdc{};
    //     // assert_eq!(Nls, from_label("nls"));
    // }
}