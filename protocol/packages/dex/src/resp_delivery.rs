use serde::Serialize;

/// The message that the integrating module should propagate to `Handler::on_inner`
pub trait ForwardToInner {
    type Msg: Serialize;

    fn msg() -> Self::Msg;
}
