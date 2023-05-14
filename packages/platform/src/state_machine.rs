use crate::message::Response as MessageResponse;

pub struct Response<State> {
    pub response: MessageResponse,
    pub next_state: State,
}

impl<State> Response<State> {
    pub fn from<R, S>(resp: R, next_state: S) -> Self
    where
        R: Into<MessageResponse>,
        S: Into<State>,
    {
        Self {
            response: resp.into(),
            next_state: next_state.into(),
        }
    }

    pub fn no_msgs<S>(next_state: S) -> Self
    where
        S: Into<State>,
    {
        Self::from(MessageResponse::default(), next_state)
    }
}

pub fn from<StateFrom, StateTo>(value: Response<StateFrom>) -> Response<StateTo>
where
    StateFrom: Into<StateTo>,
{
    let contract_state: StateTo = value.next_state.into();
    Response::from(value.response, contract_state)
}
