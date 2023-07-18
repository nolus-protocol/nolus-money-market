use serde::{Deserialize, Serialize};

use currency::{lease::LeaseGroup, lpn::Lpns, payment::PaymentGroup};
use finance::coin::CoinDTO;
use sdk::schemars::{self, JsonSchema};

pub use self::{
    open::{
        ConnectionParams, Ics20Channel, InterestPaymentSpec, LoanForm, NewLeaseContract,
        NewLeaseForm,
    },
    query::{opened, opening, paid, StateQuery, StateResponse},
};

// TODO consider defining the modules public instead of just selected items
mod open;
mod query;

pub type PaymentCoin = CoinDTO<PaymentGroup>;
pub type DownpaymentCoin = PaymentCoin;
pub type LeaseCoin = CoinDTO<LeaseGroup>;
pub type LpnCoin = CoinDTO<Lpns>;

#[derive(Serialize, Deserialize)]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Repay(),
    // it is not an enum variant to represent it as a JSON object instead of JSON string
    Close(),
    // that is a limitation of cosmjs library
    PriceAlarm(),
    TimeAlarm {},
    /// An entry point for safe delivery of a Dex response
    ///
    /// Invoked always by the same contract instance.
    DexCallback(),
    /// An entry point for safe delivery of an ICA open response, error or timeout
    ///
    /// Invoked always by the same contract instance.
    DexCallbackContinue(),
}
