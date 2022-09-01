use platform::{batch::ReplyId as BatchReplyId, generate_ids};

generate_ids! {
    pub ReplyId as BatchReplyId {
        OpenLoanReq,
    }
}
