use std::fmt::Debug;

use sdk::cosmwasm_std::{self, Binary, Event, StdError, from_json};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

pub trait EventSource<'a> {
    type EventIter: Iterator<Item = &'a Event>;

    fn events(&'a self) -> Self::EventIter;
}

pub fn any_error(events: &[Event]) -> bool {
    let maybe_attr = events
        .iter()
        .flat_map(|ev| &ev.attributes)
        .find(|atr| atr.key == "delivered");

    matches!(maybe_attr.map(|attr| attr.value.as_str()), Some("error"))
}

pub fn assert_event(actual: &[Event], expected: &Event) {
    let found = actual.iter().any(|ev| {
        expected.ty == ev.ty
            && expected
                .attributes
                .iter()
                .all(|at| ev.attributes.contains(at))
    });
    assert!(found, "Expected to find {:?} among {:?}", expected, actual);
}

pub fn parse_resp<Resp>(resp: &Option<Binary>) -> Option<Resp>
where
    Resp: DeserializeOwned,
{
    resp.as_ref()
        .map(|data| from_json(data).expect("deserialization succeed"))
}

pub fn ser_de<T, R>(obj: &T) -> Result<R, StdError>
where
    T: Serialize,
    R: Debug + for<'a> Deserialize<'a> + PartialEq,
{
    cosmwasm_std::from_json(cosmwasm_std::to_json_vec(obj).expect("serialization succeed"))
}
