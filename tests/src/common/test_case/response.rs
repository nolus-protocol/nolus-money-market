use sdk::{
    api::ProtobufAny, cosmwasm_std::Coin as CwCoin, ica::InterChainMsg,
    testing::InterChainMsgReceiver,
};

#[must_use]
#[derive(Debug)]
pub(crate) struct ResponseWithInterChainMsgs<'r, T> {
    receiver: &'r mut InterChainMsgReceiver,
    response: T,
}

impl<'r, T> ResponseWithInterChainMsgs<'r, T> {
    pub(super) fn new(receiver: &'r mut InterChainMsgReceiver, response: T) -> Self {
        Self { receiver, response }
    }

    pub fn ignore_response(self) -> ResponseWithInterChainMsgs<'r, ()> {
        ResponseWithInterChainMsgs {
            receiver: self.receiver,
            response: (),
        }
    }

    #[must_use]
    #[track_caller]
    pub fn unwrap_response(mut self) -> T {
        self.expect_empty();

        self.response
    }
}

pub(crate) trait RemoteChain {
    #[track_caller]
    fn expect_empty(&mut self);

    #[track_caller]
    fn expect_ibc_transfer(&mut self, channel: &str, sender: &str, receiver: &str) -> CwCoin;

    /// Pop the next `IbcTransfer` without asserting its parties, returning
    /// `(source_channel, sender, receiver, token)`. Used by the open flow,
    /// where the remote account (the minted StubPda) is only known after the
    /// lease state can be queried — the parties are asserted by the caller.
    #[track_caller]
    fn unwrap_ibc_transfer(&mut self) -> (String, String, String, CwCoin);

    #[track_caller]
    fn expect_submit_tx(
        &mut self,
        expected_connection_id: &str,
        expected_ica_id: &str,
    ) -> Vec<ProtobufAny>;
}

impl<T> RemoteChain for ResponseWithInterChainMsgs<'_, T> {
    #[track_caller]
    fn expect_empty(&mut self) {
        assert_eq!(self.receiver.try_recv().ok(), None);
    }

    #[track_caller]
    fn unwrap_ibc_transfer(&mut self) -> (String, String, String, CwCoin) {
        let message = self
            .receiver
            .try_recv()
            .expect("Expected message for IBC transfer!");

        if let InterChainMsg::IbcTransfer {
            source_channel,
            token,
            sender,
            receiver,
            ..
        } = message
        {
            (source_channel, sender, receiver, token)
        } else {
            panic!("Expected message for IBC transfer, got {message:?}!");
        }
    }

    #[track_caller]
    fn expect_ibc_transfer(&mut self, channel: &str, sender: &str, receiver: &str) -> CwCoin {
        let message = self
            .receiver
            .try_recv()
            .expect("Expected message for IBC transfer!");

        if let InterChainMsg::IbcTransfer {
            source_channel,
            token,
            sender: actual_sender,
            receiver: actual_receiver,
            ..
        } = message
        {
            assert_eq!(source_channel, channel);
            assert_eq!(actual_sender, sender);
            assert_eq!(actual_receiver, receiver);

            token
        } else {
            panic!("Expected message for IBC transfer, got {message:?}!");
        }
    }

    #[track_caller]
    fn expect_submit_tx(
        &mut self,
        expected_connection_id: &str,
        expected_ica_id: &str,
    ) -> Vec<ProtobufAny> {
        let message = self
            .receiver
            .try_recv()
            .expect("Expected message for submitting transactions!");

        if let InterChainMsg::SubmitTx {
            connection_id,
            interchain_account_id,
            msgs: messages,
            ..
        } = message
        {
            assert_eq!(connection_id, expected_connection_id);
            assert_eq!(interchain_account_id, expected_ica_id);

            assert!(!messages.is_empty());

            messages
        } else {
            panic!("Expected message for execution of remove transactions, got {message:?}!");
        }
    }
}
