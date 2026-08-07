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

    /// Pop the next `IbcTransfer` without asserting its parties, returning
    /// `(source_channel, sender, receiver, token)`. Used by the open flow,
    /// where the remote account (the minted StubPda) is only known after the
    /// lease state can be queried — the parties are asserted by the caller.
    /// The memo is asserted empty.
    #[track_caller]
    fn unwrap_ibc_transfer(&mut self) -> (String, String, String, CwCoin);
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

        let InterChainMsg::IbcTransfer {
            source_channel,
            token,
            sender,
            receiver,
            memo,
            ..
        } = message;

        assert_empty_memo(&memo);

        (source_channel, sender, receiver, token)
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
            memo,
            ..
        } = message;

        assert_eq!(source_channel, channel);
        assert_eq!(actual_sender, sender);
        assert_eq!(actual_receiver, receiver);
        assert_empty_memo(&memo);

        token
    }
}

#[track_caller]
fn assert_empty_memo(memo: &str) {
    assert!(
        memo.is_empty(),
        "non-empty ICS-20 memo counts against Solana's transaction size limit: {memo:?}"
    );
}
