#[cfg(feature = "impl")]
pub use self::error::Error;

pub use self::{
    account::Account,
    anomaly::{Handler as AnomalyHandler, Treatment as AnomalyTreatment},
    coins_in::SwapCoins,
    connect::{Connectable, ConnectionParams, Ics20Channel},
    error::Result as DexResult,
    slippage::{Calculator as SlippageCalculator, WithCalculator},
    swap_task::{CoinsNb, SwapOutputTask, SwapTask, WithOutputTask},
    time_alarm::TimeAlarm,
    transport::{SwapError, SwapPathSlice, SwapResult, Transport},
};

#[cfg(feature = "impl")]
pub use self::{
    enterable::Enterable,
    impl_::{
        AcceptAnyNonZeroSwap, AcceptUpToMaxSlippage, MaxSlippage, StartLocalLocalState,
        StartLocalRemoteState, StartTransferInState, StateLocalOut, StateRemoteOut, TransferOut,
        start_local_local, start_local_remote, start_remote_local,
    },
    resp_delivery::ForwardToInner,
    response::{ContinueResult, Handler, Response, Result},
    state::{Contract, ContractInSwap, Stage},
    transport::{
        IBC_TIMEOUT, RemoteLeaseTransportFactory, TransferOut as TransportOut,
        TransferOutFactory as TransportOutFactory,
    },
};

mod account;
mod anomaly;
mod coins_in;
mod connect;
#[cfg(feature = "impl")]
mod enterable;
mod error;
#[cfg(feature = "impl")]
mod impl_;
#[cfg(feature = "impl")]
mod resp_delivery;
#[cfg(feature = "impl")]
mod response;
mod slippage;
#[cfg(feature = "impl")]
mod state;

mod swap_task;

mod time_alarm;

mod transport;
