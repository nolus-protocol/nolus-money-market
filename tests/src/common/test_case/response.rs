use sdk::{cosmwasm_std::Coin as CwCoin, ica::InterChainMsg, testing::InterChainMsgReceiver};

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

    /// Consume the next IBC transfer, asserting only its `channel`, and return
    /// its `(sender, receiver, token)`. Unlike [`Self::expect_ibc_transfer`]
    /// the receiver is returned rather than asserted - the funding receiver is
    /// the per-lease `LeaseAuthority` the stand-in mints fresh, so callers that
    /// do not pin it capture it here.
    #[track_caller]
    fn take_ibc_transfer(&mut self, channel: &str) -> (String, String, CwCoin);
}

impl<T> RemoteChain for ResponseWithInterChainMsgs<'_, T> {
    #[track_caller]
    fn expect_empty(&mut self) {
        assert_eq!(self.receiver.try_recv().ok(), None);
    }

    #[track_caller]
    fn expect_ibc_transfer(&mut self, channel: &str, sender: &str, receiver: &str) -> CwCoin {
        let message = self
            .receiver
            .try_recv()
            .expect("Expected message for IBC transfer!");

        let InterChainMsg::IbcTransfer {
            source_channel,
            token,
            sender: actual_sender,
            receiver: actual_receiver,
            ..
        } = message;

        assert_eq!(source_channel, channel);
        assert_eq!(actual_sender, sender);
        assert_eq!(actual_receiver, receiver);

        token
    }

    #[track_caller]
    fn take_ibc_transfer(&mut self, channel: &str) -> (String, String, CwCoin) {
        let message = self
            .receiver
            .try_recv()
            .expect("Expected message for IBC transfer!");

        let InterChainMsg::IbcTransfer {
            source_channel,
            token,
            sender,
            receiver,
            ..
        } = message;

        assert_eq!(source_channel, channel);

        (sender, receiver, token)
    }
}
