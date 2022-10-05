use platform::{batch::ReplyId as BatchReplyId, generate_ids};

// TODO rename the rust module
generate_ids! {
    pub(crate) ReplyId as BatchReplyId {
        OpenLoanReq,
    }
}
