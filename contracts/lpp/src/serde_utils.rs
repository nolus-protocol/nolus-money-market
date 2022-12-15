use serde::{de::Error as DeserializeError, Deserialize, Deserializer};

pub(crate) mod finance {
    use finance::percent::Percent;

    use super::{Deserialize, DeserializeError, Deserializer};

    pub(crate) fn deserialize_hundred_capped<'de, D>(deserializer: D) -> Result<Percent, D::Error>
    where
        D: Deserializer<'de>,
    {
        Percent::deserialize(deserializer).and_then(|value| {
            if value <= Percent::HUNDRED {
                Ok(value)
            } else {
                Err(DeserializeError::custom(format!(
                    "Value equal or below 1000 expected, got {}!",
                    value.units()
                )))
            }
        })
    }

    #[cfg(test)]
    mod test {
        use finance::percent::Percent;

        use super::deserialize_hundred_capped;

        #[test]
        #[cfg(not(target_arch = "wasm32"))]
        fn deserialize_capped() {
            assert_eq!(
                deserialize_hundred_capped(&mut serde_json::Deserializer::from_str(
                    &serde_json::to_string(&Percent::HUNDRED).unwrap()
                ))
                .unwrap(),
                Percent::HUNDRED
            );

            assert!(
                deserialize_hundred_capped(&mut serde_json::Deserializer::from_str(
                    &serde_json::to_string(&(Percent::HUNDRED + Percent::from_permille(1)))
                        .unwrap()
                ))
                .is_err()
            );
        }
    }
}
