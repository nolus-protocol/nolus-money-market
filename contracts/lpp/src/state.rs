pub use self::config::Config;
#[cfg(any(feature = "contract", test))]
pub use self::{
    deposit::Deposit,
    loan::{Loan, LoanData},
    total::Total,
};

mod config;

#[cfg(any(feature = "contract", test))]
mod deposit;
#[cfg(any(feature = "contract", test))]
mod loan;
#[cfg(any(feature = "contract", test))]
mod total;
