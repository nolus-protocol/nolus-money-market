use std::fmt::Formatter;

/// This module implements (de-)serialation of Amount as String
/// to keep compatibility with pre-CW 2.x
use serde::{
    de::{Unexpected, Visitor},
    Deserializer, Serializer,
};

use crate::coin::Amount;

pub(super) fn serialize<S>(amount: &Amount, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&amount.to_string())
}

pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<Amount, D::Error>
where
    D: Deserializer<'de>,
{
    struct StrVisitor();
    impl<'de> Visitor<'de> for StrVisitor {
        type Value = Amount;

        fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
            formatter.write_str("\"<u128>\"")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            str::parse(v).map_err(|_| E::invalid_value(Unexpected::Str(v), &self))
        }
    }

    let visitor = StrVisitor();
    deserializer.deserialize_str(visitor)
}

#[cfg(test)]
mod test {
    use std::fmt::Debug;

    use currency::{
        test::{SuperGroupTestC1, SuperGroupTestC2},
        CurrencyDef,
    };
    use serde::{de::DeserializeOwned, Serialize};

    use crate::coin::{Amount, Coin, CoinDTO};
    use sdk::cosmwasm_std;

    #[test]
    fn serialize_deserialize() {
        serialize_deserialize_coin::<SuperGroupTestC1>(
            Amount::MIN,
            &json::<SuperGroupTestC1>(Amount::MIN),
        );
        serialize_deserialize_coin::<SuperGroupTestC1>(123, &json::<SuperGroupTestC1>(123));
        serialize_deserialize_coin::<SuperGroupTestC1>(
            Amount::MAX,
            &json::<SuperGroupTestC1>(Amount::MAX),
        );
        serialize_deserialize_coin::<SuperGroupTestC2>(
            Amount::MIN,
            &json::<SuperGroupTestC2>(Amount::MIN),
        );
        serialize_deserialize_coin::<SuperGroupTestC2>(7368953, &json::<SuperGroupTestC2>(7368953));
        serialize_deserialize_coin::<SuperGroupTestC2>(
            Amount::MAX,
            &json::<SuperGroupTestC2>(Amount::MAX),
        );
    }

    fn serialize_deserialize_coin<C>(amount: Amount, exp_txt: &str)
    where
        C: CurrencyDef + PartialEq + Debug,
    {
        let coin = CoinDTO::<C::Group>::from(Coin::<C>::new(amount));
        serialize_deserialize_impl(coin, exp_txt)
    }

    fn serialize_deserialize_impl<T>(obj: T, exp_txt: &str)
    where
        T: Serialize + DeserializeOwned + PartialEq + Debug,
    {
        let obj_bin = cosmwasm_std::to_json_vec(&obj).unwrap();
        assert_eq!(obj, cosmwasm_std::from_json(&obj_bin).unwrap());

        let obj_txt = String::from_utf8(obj_bin).unwrap();
        assert_eq!(exp_txt, obj_txt);

        assert_eq!(obj, cosmwasm_std::from_json(exp_txt.as_bytes()).unwrap());
    }

    fn json<C>(amount: Amount) -> String
    where
        C: CurrencyDef,
    {
        format!(r#"{{"amount":"{}","ticker":"{}"}}"#, amount, C::ticker(),)
    }
}
