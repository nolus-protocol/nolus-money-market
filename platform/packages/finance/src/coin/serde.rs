#[cfg(test)]
mod test {
    use std::fmt::Debug;

    use serde::{de::DeserializeOwned, Deserialize, Serialize};

    use crate::coin::Coin;
    use currency::{
        test::{SuperGroupTestC1, SuperGroupTestC2},
        Currency,
    };
    use sdk::cosmwasm_std::{from_slice, to_vec};

    #[test]
    fn serialize_deserialize() {
        serialize_deserialize_coin::<SuperGroupTestC1>(u128::MIN, r#"{"amount":"0"}"#);
        serialize_deserialize_coin::<SuperGroupTestC1>(123, r#"{"amount":"123"}"#);
        serialize_deserialize_coin::<SuperGroupTestC1>(
            u128::MAX,
            r#"{"amount":"340282366920938463463374607431768211455"}"#,
        );
        serialize_deserialize_coin::<SuperGroupTestC2>(u128::MIN, r#"{"amount":"0"}"#);
        serialize_deserialize_coin::<SuperGroupTestC2>(7368953, r#"{"amount":"7368953"}"#);
        serialize_deserialize_coin::<SuperGroupTestC2>(
            u128::MAX,
            r#"{"amount":"340282366920938463463374607431768211455"}"#,
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
        {
            coin: Coin<C>,
        }
        let coin_container = CoinContainer {
            coin: Coin::<SuperGroupTestC2>::new(10),
        };
        serialize_deserialize_impl(coin_container, r#"{"coin":{"amount":"10"}}"#);
    }

    #[test]
    fn distinct_repr() {
        let amount = 432;
        assert_eq!(
            to_vec(&Coin::<SuperGroupTestC1>::new(amount)),
            to_vec(&Coin::<SuperGroupTestC2>::new(amount))
        );
    }

    #[test]
    fn currency_tolerant() {
        let amount = 134;
        let nls_bin = to_vec(&Coin::<SuperGroupTestC1>::new(amount)).unwrap();
        let res = from_slice::<Coin<SuperGroupTestC2>>(&nls_bin);
        assert_eq!(Ok(amount.into()), res);
    }
}
