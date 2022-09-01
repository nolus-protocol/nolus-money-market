use platform::{batch::ReplyId as BatchReplyId, generate_ids};

generate_ids! {
    ReplyId as BatchReplyId {
        OpenLoanReq,
    }
}
