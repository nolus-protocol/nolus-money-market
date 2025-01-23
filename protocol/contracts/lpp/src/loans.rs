use std::{marker::PhantomData, mem};

use sdk::{
    cosmwasm_std::{Addr, Storage},
    cw_storage_plus::Map,
};

use crate::{
    contract::{ContractError, Result},
    loan::Loan,
};

pub struct Repo<Lpn>(PhantomData<Lpn>);
impl<Lpn> Repo<Lpn> {
    const STORAGE: Map<Addr, Loan<Lpn>> = Map::new("loans");

    pub fn open(storage: &mut dyn Storage, addr: Addr, loan: &Loan<Lpn>) -> Result<()> {
        if Self::STORAGE.has(storage, addr.clone()) {
            return Err(ContractError::LoanExists {});
        }

        Self::STORAGE.save(storage, addr, loan).map_err(Into::into)
    }

    pub fn load(storage: &dyn Storage, addr: Addr) -> Result<Loan<Lpn>> {
        Self::STORAGE.load(storage, addr).map_err(Into::into)
    }

    pub fn save(storage: &mut dyn Storage, addr: Addr, loan: Loan<Lpn>) -> Result<()> {
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
}

#[cfg(test)]
mod test {
    use currencies::Lpn;
    use finance::{coin::Coin, duration::Duration, percent::Percent, zero::Zero};
    use sdk::cosmwasm_std::{testing, Addr, Timestamp};

    use crate::{contract::ContractError, loan::Loan, loans::Repo};

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
        Repo::open(deps.as_mut().storage, addr.clone(), &loan).expect("should open loan");

        let result = Repo::open(deps.as_mut().storage, addr.clone(), &loan);
        assert_eq!(result, Err(ContractError::LoanExists {}));

        let mut loan: Loan<Lpn> =
            Repo::load(deps.as_ref().storage, addr.clone()).expect("should load loan");

        time = Timestamp::from_nanos(Duration::YEAR.nanos() / 2);
        let interest: Coin<Lpn> = loan.interest_due(&time);
        assert_eq!(interest, 100u128.into());
        // partial repay
        let payment = loan.repay(&time, 600u128.into());
        assert_eq!(payment.interest, 100u128.into());
        assert_eq!(payment.principal, 500u128.into());
        assert_eq!(payment.excess, 0u128.into());

        assert_eq!(loan.principal_due, 500u128.into());
        Repo::save(deps.as_mut().storage, addr.clone(), loan).unwrap();

        let mut loan: Loan<Lpn> =
            Repo::load(deps.as_ref().storage, addr.clone()).expect("should load loan");

        // repay with excess, should close the loan
        let payment = loan.repay(&time, 600u128.into());
        assert_eq!(payment.interest, 0u128.into());
        assert_eq!(payment.principal, 500u128.into());
        assert_eq!(payment.excess, 100u128.into());
        assert_eq!(loan.principal_due, Coin::ZERO);
        Repo::save(deps.as_mut().storage, addr.clone(), loan).unwrap();

        // is it cleaned up?
        let is_none = Repo::<Lpn>::query(deps.as_ref().storage, addr)
            .expect("should query loan")
            .is_none();
        assert!(is_none);
    }
}
