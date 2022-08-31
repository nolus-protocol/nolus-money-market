use cosmwasm_std::{Addr, Timestamp};
use serde::Serialize;

use finance::{
    currency::{Currency, SymbolOwned},
    price::PriceDTO
};
use lpp::stub::Lpp as LppTrait;
use platform::bank::BankAccountView;

use crate::{
    contract::alarms::{emit_events, LiquidationResult},
    error::ContractError,
    lease::{
        Lease,
        OnAlarmResult,
        WithLease,
    }
};

pub struct PriceAlarm<'a, B>
where
    B: BankAccountView,
{
    sender: &'a Addr,
    lease: Addr,
    account: B,
    now: Timestamp,
    price: PriceDTO,
}

impl<'a, B> PriceAlarm<'a, B>
where
    B: BankAccountView,
{
    pub fn new(sender: &'a Addr, lease: Addr, account: B, now: Timestamp, price: PriceDTO) -> Self {
        Self {
            sender,
            lease,
            account,
            now,
            price,
        }
    }
}

impl<'a, B> WithLease for PriceAlarm<'a, B>
where
    B: BankAccountView,
{
    type Output = LiquidationResult;

    type Error = ContractError;

    fn exec<Lpn, Lpp>(self, lease: Lease<Lpn, Lpp>) -> Result<Self::Output, Self::Error>
    where
        Lpp: LppTrait<Lpn>,
        Lpn: Currency + Serialize,
    {
        if !lease.sent_oracle(self.sender) {
            return Err(Self::Error::Unauthorized {});
        }

        let OnAlarmResult {
            batch,
            lease_dto,
            liquidation_status,
        } = lease.on_price_alarm(
            self.now,
            &self.account,
            self.lease.clone(),
            self.price.try_into()?,
        )?;

        Ok(LiquidationResult {
            response: emit_events(&liquidation_status, batch),
            lease_dto,
        })
    }

    fn unknown_lpn(self, symbol: SymbolOwned) -> Result<Self::Output, Self::Error> {
        Err(ContractError::UnknownCurrency { symbol })
    }
}
