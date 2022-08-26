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
        LiquidationStatus,
        WithLease
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

        let (liquidation, lease_amount) = lease.run_liquidation(
            self.now,
            &self.account,
            self.lease.clone(),
            self.price.try_into()?,
        )?;

        let reschedule_msgs = (
            !matches!(liquidation, LiquidationStatus::FullLiquidation(_))
        ).then(
            {
                // Force move before closure to avoid edition warning from clippy;
                let lease_addr = self.lease;

                || lease.reschedule_price_alarm(lease_addr, lease_amount, &self.now, &liquidation)
            }
        ).transpose()?;

        let (lease, lpp) = lease.into_dto();

        let mut batch = lpp.into();

        reschedule_msgs.into_iter()
            .for_each(|msg| batch.schedule_execute_batch_message(msg));

        Ok(LiquidationResult {
            into_response: emit_events(&liquidation, batch),
            lease,
        })
    }

    fn unknown_lpn(self, symbol: SymbolOwned) -> Result<Self::Output, Self::Error> {
        Err(ContractError::UnknownCurrency { symbol })
    }
}
