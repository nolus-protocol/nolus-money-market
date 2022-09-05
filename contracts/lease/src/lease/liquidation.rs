use cosmwasm_std::{Addr, Timestamp};
use serde::Serialize;

use finance::{
    coin::Coin,
    currency::{Currency, SymbolOwned},
    fraction::Fraction,
    percent::{Percent, Units},
    price::{total, total_of, Price, PriceDTO},
    ratio::Rational,
};
use lpp::stub::Lpp as LppTrait;
use market_price_oracle::stub::Oracle as OracleTrait;
use marketprice::alarms::Alarm;
use platform::{bank::BankAccountView, batch::Batch, generate_ids};
use time_alarms::stub::TimeAlarms as TimeAlarmsTrait;

use crate::{
    error::{ContractError, ContractResult},
    lease::Lease,
    loan::State,
};

use super::LeaseDTO;

impl<Lpn, Lpp, TimeAlarms, Oracle> Lease<Lpn, Lpp, TimeAlarms, Oracle>
where
    Lpn: Currency + Serialize,
    Lpp: LppTrait<Lpn>,
    TimeAlarms: TimeAlarmsTrait,
    Oracle: OracleTrait<Lpn>,
{
    pub(crate) fn on_price_alarm<B>(
        mut self,
        now: Timestamp,
        account: &B,
        lease: Addr,
        price: Price<Lpn, Lpn>,
    ) -> ContractResult<OnAlarmResult<Lpn>>
    where
        B: BankAccountView,
    {
        assert_ne!(self.currency, Lpn::SYMBOL);

        let liquidation_status = self.on_alarm(now, account, lease, price)?;

        let (lease_dto, batch) = self.into_dto();

        Ok(OnAlarmResult {
            batch,
            lease_dto,
            liquidation_status,
        })
    }

    pub(crate) fn on_time_alarm<B>(
        self,
        now: Timestamp,
        account: &B,
        lease: Addr,
    ) -> ContractResult<OnAlarmResult<Lpn>>
    where
        B: BankAccountView,
    {
        let price = if self.currency == Lpn::SYMBOL {
            total_of(Coin::new(1)).is(Coin::new(1))
        } else {
            self.oracle
                .price_of(self.currency.clone())?
                .price
                .try_into()?
        };

        self.on_price_alarm(now, account, lease, price)
    }

    fn on_alarm<B>(
        &mut self,
        now: Timestamp,
        account: &B,
        lease: Addr,
        price: Price<Lpn, Lpn>,
    ) -> ContractResult<Status<Lpn>>
    where
        B: BankAccountView,
    {
        let lease_amount = account.balance::<Lpn>()?;

        let status = self.act_on_liability(now, lease.clone(), lease_amount, price)?;

        // TODO run liquidation

        if !matches!(status, Status::FullLiquidation(_)) {
            self.reschedule(lease, lease_amount, &now, &status)?;
        }

        Ok(status)
    }

    fn act_on_liability(
        &self,
        now: Timestamp,
        lease: Addr,
        lease_amount: Coin<Lpn>,
        market_price: Price<Lpn, Lpn>,
    ) -> ContractResult<Status<Lpn>> {
        let loan_state = self.loan.state(now, lease)?;

        Ok(loan_state.map_or(Status::None, |state| {
            let lease_lpn = total(lease_amount, market_price);

            let (liability_lpn, liability) = Self::liability(state, lease_lpn);

            if self.liability.max_percent() <= liability {
                self.liquidate(
                    self.customer.clone(),
                    self.currency.clone(),
                    lease_lpn,
                    liability_lpn,
                )
            } else {
                self.handle_warnings(liability)
            }
        }))
    }

    fn handle_warnings(&self, liability: Percent) -> Status<Lpn> {
        debug_assert!(liability < self.liability.max_percent());
        if liability < self.liability.first_liq_warn_percent() {
            return Status::None;
        }

        let (ltv, level) = if self.liability.third_liq_warn_percent() <= liability {
            (self.liability.third_liq_warn_percent(), WarningLevel::Third)
        } else if self.liability.second_liq_warn_percent() <= liability {
            (
                self.liability.second_liq_warn_percent(),
                WarningLevel::Second,
            )
        } else {
            debug_assert!(self.liability.first_liq_warn_percent() <= liability);
            (self.liability.first_liq_warn_percent(), WarningLevel::First)
        };

        Status::Warning(
            LeaseInfo {
                customer: self.customer.clone(),
                ltv,
                lease_asset: self.currency.clone(),
            },
            level,
        )
    }

    fn liability_lpn(state: State<Lpn>) -> Coin<Lpn> {
        state.principal_due
            + state.previous_margin_interest_due
            + state.previous_interest_due
            + state.current_margin_interest_due
            + state.current_interest_due
    }

    fn liability(state: State<Lpn>, lease_lpn: Coin<Lpn>) -> (Coin<Lpn>, Percent) {
        let liability_lpn = Self::liability_lpn(state);

        (liability_lpn, Percent::from_ratio(liability_lpn, lease_lpn))
    }

    fn liquidate(
        &self,
        customer: Addr,
        lease_asset: SymbolOwned,
        lease_lpn: Coin<Lpn>,
        liability_lpn: Coin<Lpn>,
    ) -> Status<Lpn> {
        // from 'liability - liquidation = healthy% of (lease - liquidation)' follows
        // 'liquidation = 100% / (100% - healthy%) of (liability - healthy% of lease)'
        let multiplier = Rational::new(
            Percent::HUNDRED,
            Percent::HUNDRED - self.liability.healthy_percent(),
        );
        let extra_liability = liability_lpn - self.liability.healthy_percent().of(lease_lpn);
        let liquidation_amount =
            <Rational<Percent> as Fraction<Units>>::of(&multiplier, extra_liability);
        let liquidation_amount = lease_lpn.min(liquidation_amount);
        // TODO perform actual liquidation

        let info = LeaseInfo {
            customer,
            ltv: self.liability.max_percent(),
            lease_asset,
        };

        if liquidation_amount == lease_lpn {
            Status::FullLiquidation(info)
        } else {
            Status::PartialLiquidation {
                _info: info,
                _healthy_ltv: self.liability.healthy_percent(),
                _liquidation_amount: liquidation_amount,
            }
        }
    }

    #[inline]
    pub(super) fn initial_alarm_schedule<A>(
        &mut self,
        lease: A,
        lease_amount: Coin<Lpn>,
        now: &Timestamp,
    ) -> ContractResult<()>
    where
        A: Into<Addr>,
    {
        self.reschedule(lease, lease_amount, now, &Status::None)
    }

    #[inline]
    pub(super) fn reschedule_on_repay<A>(
        &mut self,
        lease: A,
        lease_amount: Coin<Lpn>,
        now: &Timestamp,
    ) -> ContractResult<()>
    where
        A: Into<Addr>,
    {
        self.reschedule(lease, lease_amount, now, &Status::None)
    }

    #[inline]
    fn reschedule<A>(
        &mut self,
        lease: A,
        lease_amount: Coin<Lpn>,
        now: &Timestamp,
        liquidation_status: &Status<Lpn>,
    ) -> ContractResult<()>
    where
        A: Into<Addr>,
    {
        self.reschedule_time_alarm(now)?;

        self.reschedule_price_alarm(lease, lease_amount, now, liquidation_status)
    }

    fn reschedule_price_alarm<A>(
        &mut self,
        lease: A,
        lease_amount: Coin<Lpn>,
        now: &Timestamp,
        liquidation_status: &Status<Lpn>,
    ) -> ContractResult<()>
    where
        A: Into<Addr>,
    {
        if self.currency != Lpn::SYMBOL {
            let lease = lease.into();

            let (below, above) = match liquidation_status {
                Status::None | Status::PartialLiquidation { .. } => {
                    (self.liability.first_liq_warn_percent(), None)
                }
                Status::Warning(_, WarningLevel::First) => (
                    self.liability.second_liq_warn_percent(),
                    Some(self.liability.first_liq_warn_percent()),
                ),
                Status::Warning(_, WarningLevel::Second) => (
                    self.liability.third_liq_warn_percent(),
                    Some(self.liability.second_liq_warn_percent()),
                ),
                Status::Warning(_, WarningLevel::Third) => (
                    self.liability.max_percent(),
                    Some(self.liability.third_liq_warn_percent()),
                ),
                Status::FullLiquidation(_) => unreachable!(),
            };

            let below = self.price_alarm_by_percent(lease.clone(), lease_amount, now, below)?;

            let above = above
                .map(|above| self.price_alarm_by_percent(lease, lease_amount, now, above))
                .transpose()?;

            self.oracle
                .add_alarm(Alarm::new::<PriceDTO>(
                    self.currency.clone(),
                    below.into(),
                    above.map(Into::into),
                ))
                .map_err(Into::into)
        } else {
            Ok(())
        }
    }

    fn reschedule_time_alarm(&mut self, now: &Timestamp) -> ContractResult<()> {
        self.time_alarms
            .add_alarm(*now + self.liability.recalculation_time())
            .map_err(Into::into)
    }

    fn price_alarm_by_percent<A>(
        &self,
        lease: A,
        lease_amount: Coin<Lpn>,
        now: &Timestamp,
        percent: Percent,
    ) -> ContractResult<Price<Lpn, Lpn>>
    where
        A: Into<Addr>,
    {
        let state = self
            .loan
            .state(*now + self.liability.recalculation_time(), lease.into())?
            .ok_or(ContractError::LoanClosed())?;

        assert!(!lease_amount.is_zero(), "Loan already paid!");

        Ok(total_of(percent.of(lease_amount)).is(Self::liability_lpn(state)))
    }
}

pub(crate) struct OnAlarmResult<Lpn>
where
    Lpn: Currency,
{
    pub batch: Batch,
    pub lease_dto: LeaseDTO,
    pub liquidation_status: Status<Lpn>,
}

pub(crate) enum Status<Lpn>
where
    Lpn: Currency,
{
    None,
    Warning(LeaseInfo, WarningLevel),
    PartialLiquidation {
        _info: LeaseInfo,
        _healthy_ltv: Percent,
        _liquidation_amount: Coin<Lpn>,
    },
    FullLiquidation(LeaseInfo),
}

pub(crate) struct LeaseInfo {
    pub customer: Addr,
    pub ltv: Percent,
    pub lease_asset: SymbolOwned,
}

generate_ids! {
    pub(crate) WarningLevel as u8 {
        First = 1,
        Second = 2,
        Third = 3,
    }
}

impl WarningLevel {
    pub fn to_uint(self) -> u8 {
        self.into()
    }
}
