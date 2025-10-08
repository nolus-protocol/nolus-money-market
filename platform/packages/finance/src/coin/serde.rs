#[cfg(test)]
mod test {
    use std::fmt::Debug;

    use serde::{Deserialize, Serialize, de::DeserializeOwned};

    use currency::{
        Currency,
        test::{SuperGroupTestC1, SuperGroupTestC2},
    };
    use sdk::cosmwasm_std::{StdError as CwError, from_json, to_json_vec};

    use crate::{
        coin::{Amount, Coin},
        test::coin,
    };

    #[test]
    fn serialize_deserialize() {
        serialize_deserialize_coin::<SuperGroupTestC1>(Amount::MIN, &json(Amount::MIN));
        serialize_deserialize_coin::<SuperGroupTestC1>(123, &json(123));
        serialize_deserialize_coin::<SuperGroupTestC1>(Amount::MAX, &json(Amount::MAX));
        serialize_deserialize_coin::<SuperGroupTestC2>(Amount::MIN, &json(Amount::MIN));
        serialize_deserialize_coin::<SuperGroupTestC2>(7368953, &json(7368953));
        serialize_deserialize_coin::<SuperGroupTestC2>(Amount::MAX, &json(Amount::MAX));
    }

    #[test]
    fn serialize_deserialize_as_field() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct CoinContainer<C> {
            coin: Coin<C>,
        }
        let coin_container = CoinContainer {
            coin: coin::coin2(10),
        };
        serialize_deserialize_impl(coin_container, &format!(r#"{{"coin":{}}}"#, json(10)));
    }

    #[test]
    fn distinct_repr() {
        let amount = 432;
        assert_eq!(
            to_json_vec(&coin::coin1(amount))
                .as_ref()
                .map_err(CwError::to_string),
            to_json_vec(&coin::coin2(amount))
                .as_ref()
                .map_err(CwError::to_string)
        );
    }

    #[test]
    fn currency_tolerant() {
        let amount = 134;
        let nls_bin = to_json_vec(&coin::coin1(amount)).unwrap();
        let res = from_json::<Coin<SuperGroupTestC2>>(&nls_bin);
        assert_eq!(
            Ok(&coin::coin2(amount)),
            res.as_ref().map_err(CwError::to_string)
        );
    }

    fn serialize_deserialize_coin<C>(amount: Amount, exp_txt: &str)
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
        let obj_bin = to_json_vec(&obj).unwrap();
        assert_eq!(obj, from_json(&obj_bin).unwrap());

        let obj_txt = String::from_utf8(obj_bin).unwrap();
        assert_eq!(exp_txt, obj_txt);

        assert_eq!(obj, from_json(exp_txt.as_bytes()).unwrap());
    }

    fn json(amount: Amount) -> String {
        format!(r#"{{"amount":"{amount}"}}"#)
    }
}
