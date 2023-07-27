use sdk::{
    cosmwasm_ext::InterChainMsg, cosmwasm_std::Coin as CwCoin,
    neutron_sdk::bindings::types::ProtobufAny, testing::InterChainMsgReceiver,
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
    fn expect_register_ica(&mut self, expected_connection_id: &str, expected_ica_id: &str);

    #[track_caller]
    fn expect_ibc_transfer(&mut self, channel: &str, coin: CwCoin, sender: &str, receiver: &str);

    #[track_caller]
    fn expect_submit_tx(
        &mut self,
        expected_connection_id: &str,
        expected_ica_id: &str,
        type_urls: &[&'static str],
    );
}

impl<'r, T> RemoteChain for ResponseWithInterChainMsgs<'r, T> {
    #[track_caller]
    fn expect_empty(&mut self) {
        assert_eq!(self.receiver.try_recv().ok(), None);
    }

    #[track_caller]
    fn expect_register_ica(&mut self, expected_connection_id: &str, expected_ica_id: &str) {
        let message = self
            .receiver
            .try_recv()
            .expect("Expected message for ICA registration!");

        if let InterChainMsg::RegisterInterchainAccount {
            connection_id,
            interchain_account_id,
        } = message
        {
            assert_eq!(connection_id, expected_connection_id);
            assert_eq!(interchain_account_id, expected_ica_id);
        } else {
            panic!("Expected message for ICA registration, got {message:?}!");
        }
    }

    #[track_caller]
    fn expect_ibc_transfer(&mut self, channel: &str, coin: CwCoin, sender: &str, receiver: &str) {
        let message = self
            .receiver
            .try_recv()
            .expect("Expected message for ICA registration!");

        if let InterChainMsg::IbcTransfer {
            source_channel,
            token,
            sender: actual_sender,
            receiver: actual_receiver,
            ..
        } = message
        {
            assert_eq!(source_channel, channel);
            assert_eq!(token, coin);
            assert_eq!(actual_sender, sender);
            assert_eq!(actual_receiver, receiver);
        } else {
            panic!("Expected message for ICA registration, got {message:?}!");
        }
    }

    #[track_caller]
    fn expect_submit_tx(
        &mut self,
        expected_connection_id: &str,
        expected_ica_id: &str,
        type_urls: &[&'static str],
    ) {
        assert!(!type_urls.is_empty());

        let message = self
            .receiver
            .try_recv()
            .expect("Expected message for submitting transactions!");

        if let InterChainMsg::SubmitTx {
            connection_id,
            interchain_account_id,
            msgs,
            ..
        } = message
        {
            assert_eq!(connection_id, expected_connection_id);
            assert_eq!(interchain_account_id, expected_ica_id);

            let mut index: usize = 0;

            msgs.into_iter().for_each(|msg: ProtobufAny| {
                if index == type_urls.len() {
                    panic!("More messages than provided type URLs encountered! Message's type URL is: {}", msg.type_url);
                }

                assert_eq!(
                    msg.type_url, type_urls[index],
                    "Type URL mismatch on message with index {index}"
                );

                index += 1;
            });

            assert_eq!(index, type_urls.len());
        } else {
            panic!("Expected message for ICA registration, got {message:?}!");
        }
    }
}
