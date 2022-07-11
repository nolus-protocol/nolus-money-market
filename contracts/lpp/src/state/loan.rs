use cosmwasm_std::{Timestamp, Addr, Storage, StdResult};
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use schemars::JsonSchema;
use cw_storage_plus::Map;
use crate::error::ContractError;
use std::cmp;
use finance::interest::InterestPeriod;
use finance::duration::Duration;
use finance::coin::Coin;
use finance::percent::Percent;
use finance::currency::Currency;


#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct LoanData<LPN> 
where
    LPN: Currency,
{
    pub principal_due: Coin<LPN>,
    pub annual_interest_rate: Percent,
    pub interest_paid: Timestamp,
}

pub struct Loan<LPN>
where
    LPN: Currency,
{
    addr: Addr,
    data: LoanData<LPN>,
}

impl<LPN> Loan<LPN>
where
    LPN: Currency + Serialize + DeserializeOwned,
 {
    const STORAGE: Map<'static, Addr, LoanData<LPN>> = Map::new("loans");

    pub fn open(
        storage: &mut dyn Storage,
        addr: Addr,
        amount: Coin<LPN>,
        annual_interest_rate: Percent,
        current_time: Timestamp
    ) -> Result<(), ContractError> {

        if Self::STORAGE.has(storage, addr.clone()) {
            return Err(ContractError::LoanExists {})
        }

        let data = LoanData {
            principal_due: amount,
            annual_interest_rate,
            interest_paid: current_time,
        };

        Self::STORAGE.save(storage, addr, &data)
            .map_err(ContractError::Std)
    }

    pub fn load(storage: &dyn Storage, addr: Addr) -> StdResult<Self> {
        let data = Self::STORAGE.load(storage, addr.clone())?;
        let loan = Self {
            data,
            addr,
        };

        Ok(loan)
    }

    pub fn data(&self) -> &LoanData<LPN> {
        &self.data
    }

    /// change the Loan state after repay, return (principal_payment, excess_received) pair
    pub fn repay(self, storage: &mut dyn Storage, ctime: Timestamp, repay_amount: Coin<LPN>) -> Result<(Coin<LPN>, Coin<LPN>), ContractError> {

        let time_delta = Duration::between(self.data.interest_paid, ctime);

        let (interest_period, interest_pay_excess) = InterestPeriod::with_interest(self.data.annual_interest_rate)
            .from(self.data.interest_paid)
            .spanning(time_delta)
            .pay(self.data.principal_due, repay_amount, ctime);

        let loan_principal_payment =
            cmp::min(interest_pay_excess, self.data.principal_due);
        let excess_received = interest_pay_excess - loan_principal_payment;

        if self.data.principal_due == loan_principal_payment {
            Self::STORAGE.remove(storage, self.addr);
        } else {
            Self::STORAGE.update(
                storage,
                self.addr,
                |loan| -> Result<LoanData<LPN>, ContractError> {
                    let mut loan = loan.ok_or(ContractError::NoLoan {})?;
                    loan.principal_due = loan.principal_due - loan_principal_payment;
                    loan.interest_paid = interest_period.start();

                    Ok(loan)
                },
            )?;
        }

        Ok((loan_principal_payment, excess_received))
    }

    pub fn query(
        storage: &dyn Storage,
        lease_addr: Addr,
    ) -> StdResult<Option<LoanData<LPN>>> {
        Self::STORAGE.may_load(storage, lease_addr)
    }

    pub fn query_outstanding_interest(
        storage: &dyn Storage,
        lease_addr: Addr,
        outstanding_time: Timestamp,
    ) -> StdResult<Option<Coin<LPN>>> {
        let maybe_loan = Self::STORAGE.may_load(storage, lease_addr)?;

        if let Some(loan) = maybe_loan {

            let delta_t = Duration::from_nanos(
                cmp::max(outstanding_time.nanos(), loan.interest_paid.nanos())
                - loan.interest_paid.nanos()
            );
            
            let interest_period = InterestPeriod::with_interest(loan.annual_interest_rate)
                .from(loan.interest_paid)
                .spanning(delta_t);

            let outstanding_interest_amount = interest_period.interest(loan.principal_due);

            Ok(Some(outstanding_interest_amount))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use cosmwasm_std::testing;
    use finance::{duration::Duration, currency::Usdc};
    
    #[test]
    fn test_open_and_repay_loan() {
        let mut deps = testing::mock_dependencies();

        let mut time = Timestamp::from_nanos(0);

        let addr = Addr::unchecked("leaser");
        Loan::open(deps.as_mut().storage, addr.clone(), Coin::<Usdc>::new(1000), Percent::from_percent(20), time)
            .expect("should open loan");

        let loan: Loan<Usdc> = Loan::load(deps.as_ref().storage, addr.clone())
            .expect("should load loan");

        time = Timestamp::from_nanos(Duration::YEAR.nanos()/2);
        let interest: Coin<Usdc> = Loan::query_outstanding_interest(deps.as_ref().storage, addr.clone(), time)
            .expect("should query interest")
            .expect("should be some interest");
        assert_eq!(interest, 100u128.into());

        // partial repay
        let (principal_payment, excess_received) = loan.repay(deps.as_mut().storage, time, 600u128.into())
            .expect("should repay");
        assert_eq!(principal_payment, 500u128.into());
        assert_eq!(excess_received, 0u128.into());

        let resp = Loan::<Usdc>::query(deps.as_ref().storage, addr.clone())
            .expect("should query loan")
            .expect("should be some loan");

        assert_eq!(resp.principal_due, 500u128.into());

        let loan: Loan<Usdc> = Loan::load(deps.as_ref().storage, addr.clone())
            .expect("should load loan");

        // repay with excess, should close the loan
        let (principal_payment, excess_received) = loan.repay(deps.as_mut().storage, time, 600u128.into())
            .expect("should repay");
        assert_eq!(principal_payment, 500u128.into());
        assert_eq!(excess_received, 100u128.into());

        // is it cleaned up?
        let is_none = Loan::<Usdc>::query(deps.as_ref().storage, addr)
            .expect("should query loan")
            .is_none();
        assert!(is_none);
    }
}
