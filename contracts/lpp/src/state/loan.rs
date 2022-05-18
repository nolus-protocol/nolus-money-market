use cosmwasm_std::{coin, Coin, Uint128, Decimal, Timestamp, Addr, Storage, StdResult, Env};
use serde::{Serialize, Deserialize};
use schemars::JsonSchema;
use cw_storage_plus::Map;
use crate::error::ContractError;
use crate::state::Config;
use crate::calc;
use std::cmp;


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LoanData {
    pub principal_due: Uint128,
    pub annual_interest_rate: Decimal,
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
        annual_interest_rate: Decimal,
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

        let time_delta = calc::dt(env, self.data.interest_paid);
        let loan_interest_due = calc::interest(self.data.principal_due, self.data.annual_interest_rate, time_delta);
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

                    let interest_paid_delta: u64 = (loan_interest_payment / loan_interest_due
                        * time_delta)
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

    // TODO: move config/denom stuff to a caller
    pub fn query_outstanding_interest(
        storage: &dyn Storage,
        lease_addr: Addr,
        outstanding_time: Timestamp,
    ) -> Result<Option<Coin>, ContractError> {
        let maybe_loan = Self::STORAGE.may_load(storage, lease_addr)?;
        let denom = Config::load(storage)?
            .denom;

        if let Some(loan) = maybe_loan {

            let delta_t: Uint128 = (cmp::max(outstanding_time.nanos(), loan.interest_paid.nanos())
                - loan.interest_paid.nanos())
            .into();

            let outstanding_interest_amount = calc::interest(loan.principal_due, loan.annual_interest_rate, delta_t);

            Ok(Some(coin(outstanding_interest_amount.u128(), denom)))
        } else {
            Ok(None)
        }
    }

}

