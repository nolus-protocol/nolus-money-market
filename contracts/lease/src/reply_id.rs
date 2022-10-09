use platform::{batch::ReplyId as BatchReplyId, generate_ids};

generate_ids! {
    pub(crate) ReplyId as BatchReplyId {
        OpenLoanReq,
    }
}
