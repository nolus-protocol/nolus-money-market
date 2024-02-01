use currency::Currency;
use finance::{coin::Coin, duration::Duration, zero::Zero};

/// Represent the due of a position
pub trait Due<Lpn> {
    /// The total due amount
    ///
    /// Includes the principal and due interest.
    /// The position stays open until there is some due amount.
    fn total_due(&self) -> Coin<Lpn>;

    /// When and how much due interest to collect
    ///
    /// Usually, the interest is accrued as per a fixed interest rate.
    /// If the accrued interest is not paid until some configured amount of time elapses it becomes overdue.
    /// When overdue interest amount goes above a configured minimum then the interest becomes collectable.
    fn overdue_collection(&self, min_amount: Coin<Lpn>) -> OverdueCollection<Lpn>;
}

pub enum OverdueCollection<Lpn> {
    /// No collectable overdue interest yet
    ///
    /// The period specifies in how much time the overdue will become collectable.
    /// Non-zero value.
    StartIn(Duration),

    /// The overdue amount to be collected
    ///
    /// The amounts accrued since the overdue period has started.
    Overdue(Coin<Lpn>),
}

impl<Lpn> OverdueCollection<Lpn>
where
    Lpn: Currency,
{
    pub fn start_in(&self) -> Duration {
        match self {
            OverdueCollection::StartIn(collect_in) => *collect_in,
            OverdueCollection::Overdue(_) => Duration::default(),
        }
    }

    pub fn amount(&self) -> Coin<Lpn> {
        match self {
            OverdueCollection::StartIn(_) => Coin::ZERO,
            OverdueCollection::Overdue(amount) => *amount,
        }
    }
}
