use finance::{coin::Coin, duration::Duration};

/// Due interest as a function of time
///
/// Usually, the interest is accrued as per a fixed interest rate.
/// If the accrued interest is not paid until some configured amount of time elapses and
/// the interest amount goes above a configured minimum then the interest becomes overdue.
pub trait InterestDue<Lpn> {
    /// Compute how much time is necessary for the accrued interest to become overdue
    ///
    /// If it is already overdue then return zero.
    fn time_to_get_to(&self, min_due_interest: Coin<Lpn>) -> Duration;
}
