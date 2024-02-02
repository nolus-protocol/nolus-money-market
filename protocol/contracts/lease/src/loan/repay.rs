use currency::Currency;
use finance::coin::Coin;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Receipt<C>
where
    C: Currency,
{
    overdue_margin_paid: Coin<C>,
    overdue_interest_paid: Coin<C>,
    due_margin_paid: Coin<C>,
    due_interest_paid: Coin<C>,
    principal_paid: Coin<C>,
    change: Coin<C>,
    close: bool,
}

impl<C> Receipt<C>
where
    C: Currency,
{
    pub fn new(
        overdue_interest: Coin<C>,
        overdue_margin: Coin<C>,
        due_interest: Coin<C>,
        due_margin: Coin<C>,
        principal_due: Coin<C>,
        principal_paid: Coin<C>,
        change: Coin<C>,
    ) -> Self {
        debug_assert!(
            principal_paid <= principal_due,
            "Payment exceeds principal!"
        );

        Self {
            overdue_interest_paid: overdue_interest,
            overdue_margin_paid: overdue_margin,
            due_interest_paid: due_interest,
            due_margin_paid: due_margin,
            principal_paid,
            change,
            close: principal_due == principal_paid,
        }
    }

    pub fn overdue_margin_paid(&self) -> Coin<C> {
        self.overdue_margin_paid
    }

    pub fn overdue_interest_paid(&self) -> Coin<C> {
        self.overdue_interest_paid
    }

    pub fn due_margin_paid(&self) -> Coin<C> {
        self.due_margin_paid
    }

    pub fn due_interest_paid(&self) -> Coin<C> {
        self.due_interest_paid
    }

    pub fn principal_paid(&self) -> Coin<C> {
        self.principal_paid
    }

    pub fn change(&self) -> Coin<C> {
        self.change
    }

    pub fn close(&self) -> bool {
        self.close
    }

    pub fn total(&self) -> Coin<C> {
        self.overdue_margin_paid
            + self.overdue_interest_paid
            + self.due_margin_paid
            + self.due_interest_paid
            + self.principal_paid
            + self.change
    }
}

#[cfg(test)]
mod tests {
    use currency::test::SuperGroupTestC1;
    use finance::{coin::Coin, zero::Zero};

    use crate::loan::RepayReceipt;

    type BorrowC = SuperGroupTestC1;

    #[test]
    fn pay_principal_full() {
        let principal = Coin::<BorrowC>::new(10);

        let receipt = RepayReceipt::new(
            Coin::ZERO,
            Coin::ZERO,
            Coin::ZERO,
            Coin::ZERO,
            principal,
            principal,
            Coin::ZERO,
        );

        assert_eq!(principal, receipt.principal_paid());
        assert!(receipt.close());
        assert_eq!(principal, receipt.total());
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic = "Payment exceeds principal!"]
    fn pay_principal_overpaid() {
        let principal = Coin::<BorrowC>::new(10);

        RepayReceipt::new(
            Coin::ZERO,
            Coin::ZERO,
            Coin::ZERO,
            Coin::ZERO,
            principal,
            principal + Coin::new(1),
            Coin::ZERO,
        );
    }
}
