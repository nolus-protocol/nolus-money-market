use super::{MarketAlarms, PriceResult};
use crate::ContractError;
use finance::{
    currency::{self, AnyVisitor, AnyVisitorResult, Currency},
    price::base::BasePrice,
};
use marketprice::alarms::{AlarmsIterator, PriceAlarms};
use sdk::cosmwasm_std::{Addr, Storage};
use serde::{de::DeserializeOwned, Serialize};
use swap::SwapGroup;

struct AlarmsCmd<'a, 'b, OracleBase>
where
    OracleBase: Currency,
{
    storage: &'a dyn Storage,
    price_alarms: &'static PriceAlarms<'static>,
    price: &'b BasePrice<SwapGroup, OracleBase>,
}

impl<'a, 'b, OracleBase> AnyVisitor for AlarmsCmd<'a, 'b, OracleBase>
where
    OracleBase: Currency,
{
    type Error = ContractError;
    type Output = AlarmsIterator<'a>;

    fn on<C>(self) -> AnyVisitorResult<Self>
    where
        C: Currency + Serialize + DeserializeOwned,
    {
        Ok(self
            .price_alarms
            .alarms::<C, OracleBase>(self.storage, self.price.try_into()?))
    }
}

/// Combines all alarms iterators, injecting price errors.
pub struct AlarmsFlatten<'a, I> {
    storage: &'a dyn Storage,
    prices: I,
    alarms: Option<AlarmsIterator<'a>>,
}

impl<'a, I> AlarmsFlatten<'a, I> {
    pub fn new(storage: &'a dyn Storage, prices: I) -> Self {
        Self {
            storage,
            prices,
            alarms: None,
        }
    }

    fn next_price<BaseC>(&mut self) -> Option<<Self as Iterator>::Item>
    where
        I: Iterator<Item = PriceResult<BaseC>> + 'a,
        BaseC: Currency,
    {
        self.prices.find_map(|res_price| {
            res_price
                .and_then(|price| {
                    currency::visit_any_on_ticker::<SwapGroup, _>(
                        price.base_ticker(),
                        AlarmsCmd {
                            storage: self.storage,
                            price_alarms: &MarketAlarms::PRICE_ALARMS,
                            price: &price,
                        },
                    )
                    .map(|alarms| {
                        self.alarms = Some(alarms);
                        self.alarms.as_mut().and_then(next_alarm)
                    })
                })
                .unwrap_or_else(|err| Some(Err(err)))
        })
    }
}

impl<'a, I, BaseC> Iterator for AlarmsFlatten<'a, I>
where
    I: Iterator<Item = PriceResult<BaseC>> + 'a,
    BaseC: Currency,
{
    type Item = Result<Addr, ContractError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.alarms
            .as_mut()
            .and_then(next_alarm)
            .or_else(|| self.next_price())
    }
}

fn next_alarm(iter: &mut AlarmsIterator<'_>) -> Option<Result<Addr, ContractError>> {
    iter.next().map(|res| res.map_err(ContractError::from))
}

#[cfg(test)]
mod test {
    use super::super::test::test_case;
    use super::*;
    use crate::tests;
    use ::currency::lease::{Atom, Cro, Juno, Weth};
    use sdk::cosmwasm_std::{testing::MockStorage, StdError};

    #[test]
    fn error_handling() {
        let mut storage = MockStorage::new();

        test_case(&mut storage);

        let mut alarms_iter = AlarmsFlatten::new(
            &storage,
            [
                Ok(tests::base_price::<Weth>(1, 15)),
                Ok(tests::base_price::<Cro>(1, 15)), // no alarms for this price
                Ok(tests::base_price::<Atom>(1, 25)),
                Err(StdError::generic_err("error").into()),
                Ok(tests::base_price::<Juno>(1, 25)),
            ]
            .into_iter(),
        );

        let res = alarms_iter.next();
        assert_eq!(Some(Ok(Addr::unchecked("recv2"))), res);

        // passing empty Cro iterator
        let res = alarms_iter.next();
        assert_eq!(Some(Ok(Addr::unchecked("recv4"))), res);

        // price error propagation
        let res = alarms_iter.next();
        assert_eq!(Some(Err(StdError::generic_err("error").into())), res);

        // continue after an error
        let res = alarms_iter.next();
        assert_eq!(Some(Ok(Addr::unchecked("recv5"))), res);
    }
}
