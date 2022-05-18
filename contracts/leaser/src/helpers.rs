use cosmwasm_std::Addr;
use finance::{liability::Liability, percent::Percent};
use lease::msg::{LoanForm, NewLeaseForm};

use crate::config::Config;

pub(crate) fn open_lease_msg(sender: Addr, config: Config) -> NewLeaseForm {
    NewLeaseForm {
        customer: sender.into_string(),
        currency: "UST".to_owned(), // TODO the same denom lppUST is working with
        liability: Liability::new(
            Percent::from(config.liability.initial),
            Percent::from(config.liability.healthy - config.liability.initial),
            Percent::from(config.liability.max - config.liability.healthy),
            config.recalc_hours,
        ),
        loan: LoanForm {
            annual_margin_interest_permille: config.lease_interest_rate_margin,
            lpp: config.lpp_ust_addr.into_string(),
            interest_due_period_secs: config.repayment.period_sec, // 90 days TODO use a crate for daytime calculations
            grace_period_secs: config.repayment.grace_period_sec,
        },
    }
}
