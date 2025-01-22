use std::mem;

use serde::{Deserialize, Serialize};

use finance::{coin::Coin, duration::Duration, interest, percent::Percent};
use sdk::{
    cosmwasm_std::{Addr, Storage, Timestamp},
    cw_storage_plus::Map,
    schemars::{self, JsonSchema},
};

use crate::{contract::error::Error, loan::Loan};

const STORAGE: Map<Addr, Loan<Lpn>> = Map::new("loans");

pub fn open(storage: &mut dyn Storage, addr: Addr, loan: &Self) -> Result<(), Error> {
    if Self::STORAGE.has(storage, addr.clone()) {
        return Err(Error::LoanExists {});
    }

    Self::STORAGE.save(storage, addr, loan).map_err(Into::into)
}

pub fn load(storage: &dyn Storage, addr: Addr) -> Result<Self> {
    Self::STORAGE.load(storage, addr).map_err(Into::into)
}

pub fn save(storage: &mut dyn Storage, addr: Addr, loan: Self) -> Result<()> {
    if loan.principal_due.is_zero() {
        Self::STORAGE.remove(storage, addr);
        Ok(())
    } else {
        Self::STORAGE
            .update(storage, addr, |loaded_loan| {
                let mut loaded_loan = loaded_loan.ok_or(ContractError::NoLoan {})?;
                loaded_loan.principal_due = loan.principal_due;
                loaded_loan.interest_paid = loan.interest_paid;

                Ok::<_, ContractError>(loaded_loan)
            })
            .map(mem::drop)
    }
}

pub fn query(storage: &dyn Storage, lease_addr: Addr) -> Result<Option<Loan<Lpn>>> {
    Self::STORAGE
        .may_load(storage, lease_addr)
        .map_err(Into::into)
}

#[cfg(test)]
mod test {
    use currencies::Lpn;
    use finance::{coin::Coin, duration::Duration, percent::Percent, zero::Zero};
    use sdk::cosmwasm_std::{testing, Addr, Timestamp};

    use crate::{contract::error::ContractError, loan::Loan};

    #[test]
    fn test_open_and_repay_loan() {
        let mut deps = testing::mock_dependencies();

        let mut time = Timestamp::from_nanos(0);

        let addr = Addr::unchecked("leaser");
        let loan = Loan {
            principal_due: Coin::<Lpn>::new(1000),
            annual_interest_rate: Percent::from_percent(20),
            interest_paid: time,
        };
        Loan::open(deps.as_mut().storage, addr.clone(), &loan).expect("should open loan");

        let result = Loan::open(deps.as_mut().storage, addr.clone(), &loan);
        assert_eq!(result, Err(ContractError::LoanExists {}));

        let mut loan: Loan<Lpn> =
            Loan::load(deps.as_ref().storage, addr.clone()).expect("should load loan");

        time = Timestamp::from_nanos(Duration::YEAR.nanos() / 2);
        let interest: Coin<Lpn> = loan.interest_due(&time);
        assert_eq!(interest, 100u128.into());

        // partial repay
        let payment = loan.repay(&time, 600u128.into());
        assert_eq!(payment.interest, 100u128.into());
        assert_eq!(payment.principal, 500u128.into());
        assert_eq!(payment.excess, 0u128.into());

        assert_eq!(loan.principal_due, 500u128.into());
        Loan::save(deps.as_mut().storage, addr.clone(), loan).unwrap();

        let mut loan: Loan<Lpn> =
            Loan::load(deps.as_ref().storage, addr.clone()).expect("should load loan");

        // repay with excess, should close the loan
        let payment = loan.repay(&time, 600u128.into());
        assert_eq!(payment.interest, 0u128.into());
        assert_eq!(payment.principal, 500u128.into());
        assert_eq!(payment.excess, 100u128.into());
        assert_eq!(loan.principal_due, Coin::ZERO);
        Loan::save(deps.as_mut().storage, addr.clone(), loan).unwrap();

        // is it cleaned up?
        let is_none = Loan::<Lpn>::query(deps.as_ref().storage, addr)
            .expect("should query loan")
            .is_none();
        assert!(is_none);
    }
}
