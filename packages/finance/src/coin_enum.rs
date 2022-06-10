use cosmwasm_std::Coin as CosmWasmCoin;
use serde::{Deserialize, Serialize};

pub const NLS_LABEL: &str = "unolus";
pub const USDC_LABEL: &str = "uusdc";


#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Currency {
    NLS,
    USDC,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Coin {
    amount: u128,
    currency: Currency,
}

impl Currency {
    pub fn from_label(label: &str) -> Option<Self> {
        if label == NLS_LABEL {
            Some(Currency::NLS)
        } else if label == USDC_LABEL {
            Some(Currency::USDC)
        } else {
            None
        }
    }

    pub const fn label(&self) -> &'static str {
        match self {
            Currency::NLS => NLS_LABEL,
            Currency::USDC => USDC_LABEL,
        }
    }

    pub fn coins(&self, amount: u128) -> Coin {
        Coin {
            amount,
            currency: *self,
        }
    }
}

pub fn from(coin: CosmWasmCoin) -> Option<Coin> {
    let some_currency = Currency::from_label(coin.denom.as_str());
    some_currency.map(|c| Coin {
        amount: coin.amount.into(),
        currency: c,
    })
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{to_vec, from_slice};

    use crate::coin_enum::{Currency, Coin, from, NLS_LABEL, USDC_LABEL};
    use cosmwasm_std::Coin as CosmWasmCoin;

    #[test]
    fn serialize() {
        let amount = 123;
        let coin_nls = Currency::NLS.coins(amount);
        let coin_usdc = Currency::USDC.coins(amount);

        let coin_nls_bin = to_vec(&coin_nls).unwrap();

        let coin_nls_txt = String::from_utf8(coin_nls_bin.clone()).unwrap();
        let coin_usdc_txt = String::from_utf8(to_vec(&coin_usdc).unwrap()).unwrap();
        assert_ne!(coin_nls_txt, coin_usdc_txt);

        assert_eq!(r#"{"amount":"123","currency":"USDC"}"#, coin_usdc_txt);
        assert_eq!(r#"{"amount":"123","currency":"NLS"}"#, coin_nls_txt);

        let coin_nls_deser: Coin = from_slice(&coin_nls_bin).unwrap();
        assert_eq!(coin_nls_deser, coin_nls);

    }

    #[test]
    fn from_unknown_denom() {
        assert!(from(CosmWasmCoin::new(123, "uuu")).is_none());
    }

    #[test]
    fn from_nls() {
        assert_eq!(Currency::NLS.coins(123), from(CosmWasmCoin::new(123, NLS_LABEL)).expect("nls coin"));
    }

    #[test]
    fn test_from_label() {
        assert_eq!(Some(Currency::USDC), Currency::from_label(USDC_LABEL));
        assert_eq!(Some(Currency::NLS), Currency::from_label(NLS_LABEL));
        assert_eq!(None, Currency::from_label(""));
    }
}

// vs. CosmWasm::Coin
// + Copy (store on the stack)
// + supports a list of currencies

// vs. coin_trait::Coin
// - invalid use of Add, Sub for instances of different currencies
// + easier to use + stack based