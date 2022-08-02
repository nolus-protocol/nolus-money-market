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

#[derive(Debug, Default, Clone, Eq, PartialEq)]
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

#[cfg(test)]
mod tests {
    use finance::{
        coin::Coin,
        currency::Nls
    };

    use crate::loan::LoanInterestsPaid;

    fn pay_margin_once() {
        let amount = Coin::<Nls>::new(5);

        let mut paid = LoanInterestsPaid::default();

        paid.pay_next_margin(amount);

        assert_eq!(
            paid,
            LoanInterestsPaid {
                current_margin_paid: amount,
                .. Default::default()
            },
        );
    }

    #[test]
    fn test_pay_margin_once() {
        pay_margin_once();
    }

    fn pay_margin_twice() {
        let amount_1 = Coin::<Nls>::new(5);

        let amount_2 = Coin::<Nls>::new(15);

        let mut paid = LoanInterestsPaid::default();

        paid.pay_next_margin(amount_1);

        paid.pay_next_margin(amount_2);

        assert_eq!(
            paid,
            LoanInterestsPaid {
                previous_margin_paid: amount_1,
                current_margin_paid: amount_2,
                .. Default::default()
            },
        );
    }

    #[test]
    fn test_pay_margin_twice() {
        pay_margin_twice();
    }

    fn pay_interest_once() {
        let amount = Coin::<Nls>::new(5);

        let mut paid = LoanInterestsPaid::default();

        paid.pay_next_interest(amount);

        assert_eq!(
            paid,
            LoanInterestsPaid {
                current_interest_paid: amount,
                .. Default::default()
            },
        );
    }

    #[test]
    fn test_pay_interest_once() {
        pay_interest_once();
    }

    fn pay_interest_twice() {
        let amount_1 = Coin::<Nls>::new(5);

        let amount_2 = Coin::<Nls>::new(15);

        let mut paid = LoanInterestsPaid::default();

        paid.pay_next_interest(amount_1);

        paid.pay_next_interest(amount_2);

        assert_eq!(
            paid,
            LoanInterestsPaid {
                previous_interest_paid: amount_1,
                current_interest_paid: amount_2,
                .. Default::default()
            },
        );
    }

    #[test]
    fn test_pay_interest_twice() {
        pay_interest_twice();
    }

    fn pay_principal_part() {
        let principal = Coin::<Nls>::new(10);

        let amount = Coin::<Nls>::new(5);

        let mut paid = LoanInterestsPaid::default();

        paid.pay_principal(principal, amount);

        assert_eq!(
            paid,
            LoanInterestsPaid {
                principal_paid: amount,
                .. Default::default()
            },
        );
    }

    #[test]
    fn test_pay_principal_part() {
        pay_principal_part();
    }

    fn pay_principal_full() {
        let principal = Coin::<Nls>::new(10);

        let mut paid = LoanInterestsPaid::default();

        paid.pay_principal(principal, principal);

        assert_eq!(
            paid,
            LoanInterestsPaid {
                principal_paid: principal,
                close: true,
                .. Default::default()
            },
        );
    }

    #[test]
    fn test_pay_principal_full() {
        pay_principal_full();
    }
}
