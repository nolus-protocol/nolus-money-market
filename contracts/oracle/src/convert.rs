use std::marker::PhantomData;

use cosmwasm_std::QuerierWrapper;

use finance::{
    coin::Coin,
    currency::{Currency, SymbolOwned},
    price::{self, Price},
};

use crate::{
    error,
    stub::{Oracle, OracleRef, WithOracle},
    ContractError,
};

pub fn to_base<BaseC, InC>(
    oracle_ref: OracleRef,
    in_amount: Coin<InC>,
    querier: &QuerierWrapper,
) -> Result<Coin<BaseC>, ContractError>
where
    BaseC: Currency,
    InC: Currency,
{
    struct PriceConvert<BaseC, In>
    where
        BaseC: Currency,
        In: Currency,
    {
        in_amount: Coin<In>,
        _out: PhantomData<BaseC>,
    }

    impl<BaseC, In> WithOracle<BaseC> for PriceConvert<BaseC, In>
    where
        BaseC: Currency,
        In: Currency,
    {
        type Output = Coin<BaseC>;
        type Error = ContractError;

        fn exec<OracleImpl>(self, oracle: OracleImpl) -> Result<Self::Output, Self::Error>
        where
            OracleImpl: Oracle<BaseC>,
        {
            Ok(price::total(self.in_amount, price_of(&oracle)?))
        }

        fn unexpected_base(self, found: SymbolOwned) -> Result<Self::Output, Self::Error> {
            Err(error::currency_mismatch::<BaseC>(found))
        }
    }

    oracle_ref.execute(
        PriceConvert {
            in_amount,
            _out: PhantomData,
        },
        querier,
    )
}

pub fn from_base<BaseC, OutC>(
    oracle_ref: OracleRef,
    in_amount: Coin<BaseC>,
    querier: &QuerierWrapper,
) -> Result<Coin<OutC>, ContractError>
where
    BaseC: Currency,
    OutC: Currency,
{
    struct PriceConvert<BaseC, Out>
    where
        BaseC: Currency,
        Out: Currency,
    {
        in_amount: Coin<BaseC>,
        _out: PhantomData<Out>,
    }

    impl<BaseC, Out> WithOracle<BaseC> for PriceConvert<BaseC, Out>
    where
        BaseC: Currency,
        Out: Currency,
    {
        type Output = Coin<Out>;
        type Error = ContractError;

        fn exec<OracleImpl>(self, oracle: OracleImpl) -> Result<Self::Output, Self::Error>
        where
            OracleImpl: Oracle<BaseC>,
        {
            Ok(price::total(self.in_amount, price_of(&oracle)?.inv()))
        }

        fn unexpected_base(self, found: SymbolOwned) -> Result<Self::Output, Self::Error> {
            Err(error::currency_mismatch::<BaseC>(found))
        }
    }

    oracle_ref.execute(
        PriceConvert {
            in_amount,
            _out: PhantomData,
        },
        querier,
    )
}

fn price_of<BaseC, OtherC, OracleImpl>(
    oracle: &OracleImpl,
) -> Result<Price<OtherC, BaseC>, ContractError>
where
    BaseC: Currency,
    OtherC: Currency,
    OracleImpl: Oracle<BaseC>,
{
    let price_other_to_base = oracle.price_of(OtherC::SYMBOL.to_string())?.price;
    Ok(Price::<OtherC, BaseC>::try_from(price_other_to_base)?)
}
