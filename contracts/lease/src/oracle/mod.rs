use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use cosmwasm_std::Addr;
use serde::Serialize;

use finance::currency::Currency;
use market_price_oracle::stub::{Oracle as OracleTrait, OracleRef};
use platform::batch::Batch;

pub struct Oracle<OracleC, Oracle> {
    oracle_ref: OracleRef,
    _oracle_c: PhantomData<OracleC>,
    oracle: Oracle,
}

impl<OracleC, Oracle> self::Oracle<OracleC, Oracle>
where
    OracleC: Currency + Serialize,
    Oracle: OracleTrait<OracleC>,
{
    pub fn from_dto(dto: OracleRef, oracle: Oracle) -> Self {
        Self {
            oracle_ref: dto,
            _oracle_c: PhantomData,
            oracle,
        }
    }

    pub fn into_dto(self) -> (OracleRef, Batch) {
        (self.oracle_ref, self.oracle.into())
    }

    pub fn owned_by(&self, addr: &Addr) -> bool {
        self.oracle_ref.owned_by(addr)
    }
}

impl<OracleC, Oracle> Deref for self::Oracle<OracleC, Oracle>
where
    OracleC: Currency + Serialize,
    Oracle: OracleTrait<OracleC>,
{
    type Target = Oracle;

    fn deref(&self) -> &Self::Target {
        &self.oracle
    }
}

impl<OracleC, Oracle> DerefMut for self::Oracle<OracleC, Oracle>
where
    OracleC: Currency + Serialize,
    Oracle: OracleTrait<OracleC>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.oracle
    }
}

impl<OracleC, Oracle> From<self::Oracle<OracleC, Oracle>> for Batch
where
    OracleC: Currency + Serialize,
    Oracle: OracleTrait<OracleC>,
{
    fn from(oracle: self::Oracle<OracleC, Oracle>) -> Self {
        oracle.oracle.into()
    }
}
