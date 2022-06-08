use cosmwasm_std::{Uint128, Timestamp, Addr, Storage, StdResult, Env, Decimal};
use serde::{Serialize, Deserialize};
use schemars::JsonSchema;
use cw_storage_plus::Map;
use crate::error::ContractError;
use std::cmp;
use finance::percent::Percent;
use finance::interest::InterestPeriod;
use finance::duration::Duration;


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LoanData {
    pub principal_due: Uint128,
    pub annual_interest_rate: Percent,
    pub interest_paid: Timestamp,
}

pub struct Loan {
    addr: Addr,
    data: LoanData,
}

impl Loan {
    const STORAGE: Map<'static, Addr, LoanData> = Map::new("loans");

    pub fn open(
        storage: &mut dyn Storage,
        addr: Addr,
        amount: Uint128,
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

    pub fn data(&self) -> &LoanData {
        &self.data
    }

    /// change the Loan state after repay, return (principal_payment, excess_received) pair
    pub fn repay(&mut self, storage: &mut dyn Storage, env: &Env, repay_amount: Uint128) -> Result<(Uint128, Uint128), ContractError> {

        let time_delta = Duration::between(self.data.interest_paid, env.block.time);
        let loan_interest_due = InterestPeriod::with_interest(self.data.annual_interest_rate)
            .from(self.data.interest_paid)
            .spanning(time_delta)
            .interest(self.data.principal_due);


        // let loan_interest_due = calc::interest(self.data.principal_due, self.data.annual_interest_rate, time_delta);

        let loan_interest_payment = cmp::min(loan_interest_due, repay_amount);
        let loan_principal_payment =
            cmp::min(repay_amount - loan_interest_payment, self.data.principal_due);
        let excess_received = repay_amount - loan_interest_payment - loan_principal_payment;

        if self.data.principal_due == loan_principal_payment {
            Self::STORAGE.remove(storage, self.addr.clone());
        } else {
            Self::STORAGE.update(
                storage,
                self.addr.clone(),
                |loan| -> Result<LoanData, ContractError> {
                    let mut loan = loan.ok_or(ContractError::NoLoan {})?;
                    loan.principal_due -= loan_principal_payment;

                    // TODO: use InterestPeriod::pay
                    let interest_paid_delta: u64 = (
                        Decimal::from_ratio(loan_interest_payment, loan_interest_due)
                        * Uint128::new(time_delta.nanos() as u128))
                        .u128()
                        .try_into()
                        .expect("math overflow");
                    loan.interest_paid =
                        Timestamp::from_nanos(loan.interest_paid.nanos() + interest_paid_delta);

                    Ok(loan)
                },
            )?;
        }

        Ok((loan_principal_payment, excess_received))
    }

    pub fn query(
        storage: &dyn Storage,
        lease_addr: Addr,
    ) -> StdResult<Option<LoanData>> {
        Self::STORAGE.may_load(storage, lease_addr)
    }

    pub fn query_outstanding_interest(
        storage: &dyn Storage,
        lease_addr: Addr,
        outstanding_time: Timestamp,
    ) -> StdResult<Option<Uint128>> {
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

