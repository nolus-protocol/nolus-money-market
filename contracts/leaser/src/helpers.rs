use cosmwasm_std::Addr;
use finance::{liability::Liability, percent::Percent};
use lease::msg::{LoanForm, NewLeaseForm};

use crate::config::Config;

pub(crate) fn open_lease_msg(sender: Addr, config: Config, currency: String) -> NewLeaseForm {
    NewLeaseForm {
        customer: sender.into_string(),
        currency, // TODO the same denom lppUST is working with
        liability: Liability::new(
            Percent::from_percent(config.liability.initial.into()),
            Percent::from_percent((config.liability.healthy - config.liability.initial).into()),
            Percent::from_percent((config.liability.max - config.liability.healthy).into()),
            config.recalc_hours,
        ),
        loan: LoanForm {
            annual_margin_interest: config.lease_interest_rate_margin,
            lpp: config.lpp_ust_addr.into_string(),
            interest_due_period_secs: config.repayment.period_sec, // 90 days TODO use a crate for daytime calculations
            grace_period_secs: config.repayment.grace_period_sec,
        },
    }
}
