use std::marker::PhantomData;

use cosmwasm_std::QuerierWrapper;
use finance::{
    coin::Coin,
    currency::{Currency, SymbolOwned},
    price::{self, Price},
};
use serde::Serialize;

use crate::{
    stub::{Oracle, OracleRef, WithOracle},
    ContractError,
};

pub fn from_base<BaseC, OutC>(
    oracle_ref: OracleRef,
    in_amount: Coin<BaseC>,
    querier: &QuerierWrapper,
) -> Result<Coin<OutC>, ContractError>
where
    BaseC: Currency + Serialize,
    OutC: Currency,
{
    struct PriceConvert<BaseC, Out>
    where
        BaseC: Currency,
        Out: Currency,
    {
        base_amount: Coin<BaseC>,
        _out: PhantomData<Out>,
    }

    impl<BaseC, Out> WithOracle<BaseC> for PriceConvert<BaseC, Out>
    where
        BaseC: Currency + Serialize,
        Out: Currency,
    {
        type Output = Coin<Out>;
        type Error = ContractError;

        fn exec<OracleImpl>(self, oracle: OracleImpl) -> Result<Self::Output, Self::Error>
        where
            OracleImpl: Oracle<BaseC>,
        {
            let price_out_base = oracle.price_of(Out::SYMBOL.to_string())?.price;
            let price_base_out = Price::<Out, BaseC>::try_from(price_out_base)?.inv();
            let out_amount: Coin<Out> = price::total(self.base_amount, price_base_out);
            Ok(out_amount)
        }

        fn unexpected_base(self, found: SymbolOwned) -> Result<Self::Output, Self::Error> {
            Err(ContractError::CurrencyMismatch {
                expected: BaseC::SYMBOL.into(),
                found,
            })
        }
    }

    oracle_ref.execute(
        PriceConvert {
            base_amount: in_amount,
            _out: PhantomData,
        },
        querier,
    )
}
