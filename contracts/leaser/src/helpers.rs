use cosmwasm_std::{Addr, Coin};
use finance::{liability::Liability, percent::Percent};
use lease::opening::{LoanForm, NewLeaseForm};

use crate::{config::Config, ContractError};

pub fn assert_sent_sufficient_coin(
    sent: &[Coin],
    required: Option<Coin>,
) -> Result<(), ContractError> {
    if let Some(required_coin) = required {
        let required_amount = required_coin.amount.u128();
        if required_amount > 0 {
            let sent_sufficient_funds = sent.iter().any(|coin| {
                // check if a given sent coin matches denom
                // and has sufficient amount
                coin.denom == required_coin.denom && coin.amount.u128() >= required_amount
            });

            if sent_sufficient_funds {
                return Ok(());
            } else {
                return Err(ContractError::InsufficientFundsSend {});
            }
        }
    }
    Ok(())
}

pub(crate) fn open_lease_msg(sender: Addr, config: Config) -> NewLeaseForm {
    NewLeaseForm {
        customer: sender.into_string(),
        currency: "".to_owned(), // TODO the same denom lppUST is working with
        liability: Liability::new(
            Percent::from(config.liability.initial),
            Percent::from(config.liability.healthy - config.liability.initial),
            Percent::from(config.liability.max - config.liability.healthy),
            20 * 24,
        ), //TODO
        loan: LoanForm {
            annual_margin_interest_permille: config.lease_interest_rate_margin,
            lpp: config.lpp_ust_addr.into_string(),
            interest_due_period_secs: config.repayment.period_sec, // 90 days TODO use a crate for daytime calculations
            grace_period_secs: config.repayment.grace_period_sec,
        },
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use cosmwasm_std::{coin, coins};

    #[test]
    fn assert_sent_sufficient_coin_works() {
        match assert_sent_sufficient_coin(&[], Some(coin(0, "token"))) {
            Ok(()) => {}
            Err(e) => panic!("Unexpected error: {:?}", e),
        };

        match assert_sent_sufficient_coin(&[], Some(coin(5, "token"))) {
            Ok(()) => panic!("Should have raised insufficient funds error"),
            Err(ContractError::InsufficientFundsSend {}) => {}
            Err(e) => panic!("Unexpected error: {:?}", e),
        };

        match assert_sent_sufficient_coin(&coins(10, "smokin"), Some(coin(5, "token"))) {
            Ok(()) => panic!("Should have raised insufficient funds error"),
            Err(ContractError::InsufficientFundsSend {}) => {}
            Err(e) => panic!("Unexpected error: {:?}", e),
        };

        match assert_sent_sufficient_coin(&coins(10, "token"), Some(coin(5, "token"))) {
            Ok(()) => {}
            Err(e) => panic!("Unexpected error: {:?}", e),
        };

        let sent_coins = vec![coin(2, "smokin"), coin(5, "token"), coin(1, "earth")];
        match assert_sent_sufficient_coin(&sent_coins, Some(coin(5, "token"))) {
            Ok(()) => {}
            Err(e) => panic!("Unexpected error: {:?}", e),
        };
    }
}
