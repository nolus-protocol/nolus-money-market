use std::mem::replace;

use finance::{
    coin::Coin,
    currency::Currency
};
use platform::batch::Batch;

pub(crate) struct Result<C>
where
    C: Currency,
{
    pub batch: Batch,
    pub paid: LoanInterestsPaid<C>,
}

#[derive(Default)]
pub(crate) struct LoanInterestsPaid<C>
where
    C: Currency,
{
    previous_margin_paid: Coin<C>,
    current_margin_paid: Coin<C>,
    previous_interest_paid: Coin<C>,
    current_interest_paid: Coin<C>,
    principal_paid: Coin<C>,
    close: bool,
}

impl<C> LoanInterestsPaid<C>
where
    C: Currency,
{
    pub const fn previous_margin_paid(&self) -> Coin<C> {
        self.previous_margin_paid
    }

    pub const fn previous_interest_paid(&self) -> Coin<C> {
        self.previous_interest_paid
    }

    pub const fn current_margin_paid(&self) -> Coin<C> {
        self.current_margin_paid
    }

    pub const fn current_interest_paid(&self) -> Coin<C> {
        self.current_interest_paid
    }

    pub const fn principal_paid(&self) -> Coin<C> {
        self.principal_paid
    }

    pub const fn close(&self) -> bool {
        self.close
    }

    pub(super) fn next_payment(previous: &mut Coin<C>, current: &mut Coin<C>, payment: Coin<C>) {
        *previous = replace(current, payment);
    }

    pub(super) fn pay_next_margin(&mut self, payment: Coin<C>) {
        Self::next_payment(
            &mut self.previous_margin_paid,
            &mut self.current_margin_paid,
            payment,
        );
    }

    pub(super) fn pay_next_interest(&mut self, payment: Coin<C>) {
        Self::next_payment(
            &mut self.previous_interest_paid,
            &mut self.current_interest_paid,
            payment,
        );
    }

    pub(super) fn pay_principal(&mut self, principal: Coin<C>, payment: Coin<C>) {
        self.principal_paid = payment;

        self.close = principal == payment;
    }
}
