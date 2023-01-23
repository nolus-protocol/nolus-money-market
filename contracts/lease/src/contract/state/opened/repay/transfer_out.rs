use cosmwasm_std::{Addr, Deps, DepsMut, Env, Timestamp};
use currency::native::Nls;
use finance::coin::Coin;
use platform::bank_ibc::local::{Sender, IBC_TRANSFER_TIMEOUT};
use platform::batch::Batch;
use sdk::neutron_sdk::sudo::msg::SudoMsg;
use serde::{Deserialize, Serialize};

use crate::api::opened::{OngoingTrx, RepayTrx};
use crate::api::{PaymentCoin, StateQuery, StateResponse};
use crate::contract::cmd::LeaseState;
use crate::contract::state::{Controller, Response};
use crate::error::ContractResult;
use crate::lease::{with_lease, LeaseDTO};

//TODO take them as input from the client
const ICA_TRANSFER_ACK_TIP: Coin<Nls> = Coin::new(1);
const ICA_TRANSFER_TIMEOUT_TIP: Coin<Nls> = ICA_TRANSFER_ACK_TIP;

#[derive(Serialize, Deserialize)]
pub struct TransferOut {
    lease: LeaseDTO,
    payment: PaymentCoin,
}

impl TransferOut {
    //TODO change to super or crate::contract::state::opening once the other opening states have moved to opening module
    pub(in crate::contract::state) fn new(lease: LeaseDTO, payment: PaymentCoin) -> Self {
        Self { lease, payment }
    }

    pub(in crate::contract::state) fn enter_state(
        &self,
        _sender: Addr,
        _now: Timestamp,
    ) -> ContractResult<Batch> {
        #[allow(unreachable_code)]
        let mut _ibc_sender = Sender::new(
            "channel TODO",
            _sender,
            "receiver TODO".to_owned().try_into()?,
            _now + IBC_TRANSFER_TIMEOUT,
            ICA_TRANSFER_ACK_TIP,
            ICA_TRANSFER_TIMEOUT_TIP,
        );
        // TODO apply nls_swap_fee on the downpayment only!
        _ibc_sender.send(&self.payment)?;

        Ok(_ibc_sender.into())
    }
}

impl Controller for TransferOut {
    fn sudo(self, _deps: &mut DepsMut, _env: Env, msg: SudoMsg) -> ContractResult<Response> {
        match msg {
            SudoMsg::Response {
                request: _,
                data: _,
            } => {
                todo!(
                    "proceed with Swap - TransferIn before landing to the same Lease::repay call"
                );
            }
            SudoMsg::Timeout { request: _ } => todo!(),
            SudoMsg::Error {
                request: _,
                details: _,
            } => todo!(),
            _ => todo!(),
        }
    }

    fn query(self, deps: Deps, env: Env, _msg: StateQuery) -> ContractResult<StateResponse> {
        let _ongoing = OngoingTrx::Repayment {
            payment: self.payment,
            in_progress: RepayTrx::TransferOut,
        };
        // TODO pass ongoing to the LeaseState cmd for adding it to the state
        with_lease::execute(
            self.lease,
            LeaseState::new(env.block.time),
            &env.contract.address,
            &deps.querier,
        )
    }
}
