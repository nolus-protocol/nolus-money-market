use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use currency::Group;
use finance::{
    coin::{Coin, CoinDTO},
    fraction::Fraction,
    percent::Percent100,
};
use oracle::stub::CoinToOut;
use platform::batch::{Emit, Emitter};
use sdk::cosmwasm_std::QuerierWrapper;

use crate::{
    SlippageCalculator,
    error::{Error, Result},
};

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "testing", derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct MaxSlippage(Percent100);

impl MaxSlippage {
    #[cfg(feature = "testing")]
    pub const fn unchecked(max: Percent100) -> Self {
        Self(max)
    }

    pub fn emit<Key>(&self, emitter: Emitter, key: Key) -> Emitter
    where
        Key: Into<String>,
    {
        emitter.emit_percent_amount(key, self.0)
    }

    pub fn min_out<C>(&self, amount_in: Coin<C>) -> Coin<C> {
        self.0.complement().of(amount_in)
    }
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "testing", derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Calculator<InG, ConverterT>
where
    InG: Group,
{
    max_slippage: MaxSlippage,
    converter: ConverterT,
    #[serde(skip)]
    _in_g: PhantomData<InG>,
}

impl<InG, ConverterT> Calculator<InG, ConverterT>
where
    InG: Group,
{
    pub const fn new(max_slippage: MaxSlippage, converter: ConverterT) -> Self {
        Self {
            max_slippage,
            converter,
            _in_g: PhantomData,
        }
    }

    pub const fn threshold(&self) -> MaxSlippage {
        self.max_slippage
    }
}

impl<InG, ConverterT> SlippageCalculator<InG> for Calculator<InG, ConverterT>
where
    InG: Group,
    ConverterT: CoinToOut<InG>,
{
    type OutC = ConverterT::OutC;

    fn min_output(
        &self,
        input: &CoinDTO<InG>,
        querier: QuerierWrapper<'_>,
    ) -> Result<Coin<Self::OutC>> {
        self.converter
            .to_out(input, querier)
            .map_err(Error::MinOutput)
            .map(|input_in_out_c| self.max_slippage.min_out(input_in_out_c))
    }
}

#[cfg(test)]
mod test {
    use currency::test::{SuperGroup, SuperGroupTestC1};
    use finance::{
        coin::{Amount, Coin},
        fraction::Fraction,
        percent::Percent100,
    };
    use oracle::stub::ToQuote;
    use oracle_platform::OracleRef;
    use sdk::cosmwasm_std::{Addr, from_json, to_json_string};

    use super::{Calculator, MaxSlippage};

    #[test]
    fn zero() {
        assert!(calc_min_out(coin(456), Percent100::from_percent(100)).is_zero());
    }

    #[test]
    fn hundred() {
        let coin_in = coin(100);
        assert_eq!(coin_in, calc_min_out(coin_in, Percent100::ZERO));
    }

    #[test]
    fn eighty_five() {
        let coin_in = coin(267);
        let slippage = Percent100::from_percent(15);
        assert_eq!(
            slippage.complement().of(coin_in),
            calc_min_out(coin_in, slippage)
        );
    }

    #[test]
    fn to_quote_calc_wire_shape() {
        const WIRE: &str = r#"{"max_slippage":150,"converter":{"addr":"oracle_addr"}}"#;

        let max_slippage = MaxSlippage(Percent100::from_percent(15));
        let oracle =
            OracleRef::<SuperGroupTestC1, SuperGroup>::unchecked(Addr::unchecked("oracle_addr"));
        let calc: Calculator<SuperGroup, ToQuote<SuperGroupTestC1, SuperGroup>> =
            Calculator::new(max_slippage, ToQuote::new(oracle));

        assert_eq!(WIRE, to_json_string(&calc).unwrap());

        let from_wire: Calculator<SuperGroup, ToQuote<SuperGroupTestC1, SuperGroup>> =
            from_json(WIRE.as_bytes()).unwrap();
        assert_eq!(WIRE, to_json_string(&from_wire).unwrap());
    }

    fn coin(amount: Amount) -> Coin<SuperGroupTestC1> {
        Coin::new(amount)
    }

    fn calc_min_out(
        amount_in: Coin<SuperGroupTestC1>,
        slippage: Percent100,
    ) -> Coin<SuperGroupTestC1> {
        MaxSlippage(slippage).min_out(amount_in)
    }
}
