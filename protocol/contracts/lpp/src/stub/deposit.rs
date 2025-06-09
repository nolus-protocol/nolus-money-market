use thiserror::Error;

use currency::Group;
use finance::coin::Coin;
use lpp_platform::NLpn;
use platform::batch::Batch;

use crate::msg::ExecuteMsg;

use super::LppRef;

pub trait Depositer<Lpn>
where
    Self: Into<Batch>,
{
    fn deposit(&mut self, amount: Coin<Lpn>) -> Result<(), Error>;

    fn burn(&self, amount: Coin<NLpn>) -> Result<(), Error>;

    fn close_all(&mut self) -> Result<(), Error>;
}

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Lpp][Deposit] Failed to schedule a message! Cause: {0}")]
    ScheduleMessage(platform::error::Error),
}

pub trait WithDepositer<Lpn, Lpns>
where
    Lpns: Group,
{
    type Output;
    type Error;

    fn exec<Lpp>(self, lpp: Lpp) -> Result<Self::Output, Self::Error>
    where
        Lpp: Depositer<Lpn>;
}

pub struct Impl<Lpn, Lpns> {
    lpp_ref: LppRef<Lpn, Lpns>,
    batch: Batch,
}

impl<Lpn, Lpns> Impl<Lpn, Lpns> {
    pub(super) fn new(lpp_ref: LppRef<Lpn, Lpns>) -> Self {
        Self {
            lpp_ref,
            batch: Batch::default(),
        }
    }
}

impl<Lpn, Lpns> Depositer<Lpn> for Impl<Lpn, Lpns>
where
    Lpns: Group,
{
    fn deposit(&mut self, _amount: Coin<Lpn>) -> Result<(), Error> {
        todo!()
    }

    fn burn(&self, _amount: Coin<NLpn>) -> Result<(), Error> {
        todo!()
    }

    fn close_all(&mut self) -> Result<(), Error> {
        self.batch
            .schedule_execute_wasm_no_reply_no_funds(
                self.lpp_ref.addr.clone(),
                &ExecuteMsg::<Lpns>::CloseAllDeposits(),
            )
            .map_err(Error::ScheduleMessage)
    }
}

impl<Lpn, Lpns> From<Impl<Lpn, Lpns>> for Batch {
    fn from(value: Impl<Lpn, Lpns>) -> Self {
        value.batch
    }
}
