use ::currency::payment::PaymentGroup;
use finance::{
    coin::CoinDTO,
    currency::{self, AnyVisitor, Currency, Group},
};
use oracle::msg::SwapPathResponse;
use osmosis_std::types::{
    cosmos::base::v1beta1::Coin, osmosis::gamm::v1beta1::MsgSwapExactAmountIn,
};
use platform::ica::Batch;
use sdk::cosmwasm_std::Addr;

use crate::error::{Error, Result};

pub mod error;

pub fn exact_amount_in(
    batch: &mut Batch,
    sender: &Addr,
    token_in: &CoinDTO,
    _swap_path: &SwapPathResponse,
) -> Result<()> {
    const MSG_TYPE: &str = "/osmosis.gamm.v1beta1.MsgSwapExactAmountIn";
    let routes = vec![];
    let token_in = Some(into_coin::<SwapGroup>(token_in)?);
    let token_out_min_amount = "0".into();
    let msg = MsgSwapExactAmountIn {
        sender: sender.into(),
        routes,
        token_in,
        token_out_min_amount,
    };

    batch.add_message(MSG_TYPE, msg)?;
    Ok(())
}

type SwapGroup = PaymentGroup;

fn into_coin<G>(token: &CoinDTO) -> Result<Coin>
where
    G: Group,
{
    struct CoinFactory<'c> {
        token: &'c CoinDTO,
    }
    impl<'c> AnyVisitor for CoinFactory<'c> {
        type Output = Coin;
        type Error = Error;

        fn on<C>(self) -> Result<Self::Output>
        where
            C: Currency,
        {
            Ok(Self::Output {
                denom: C::DEX_SYMBOL.into(),
                amount: self.token.amount().to_string(),
            })
        }
    }
    currency::visit_any_on_ticker::<G, _>(token.ticker(), CoinFactory { token })
}

#[cfg(test)]
mod test {
    use currency::lpn::Usdc;
    use finance::{
        coin::{Amount, Coin as FinanceCoin, CoinDTO},
        currency::Currency,
    };
    use osmosis_std::types::cosmos::base::v1beta1::Coin;

    use crate::SwapGroup;

    #[test]
    fn into_coin() {
        type Currency = Usdc;
        const AMOUNT: Amount = 243;
        let token = FinanceCoin::<Currency>::new(AMOUNT);
        let token_dto: CoinDTO = token.into();
        assert_eq!(
            Ok(Coin {
                denom: Currency::DEX_SYMBOL.into(),
                amount: AMOUNT.to_string()
            }),
            super::into_coin::<SwapGroup>(&token_dto)
        );
    }
}
