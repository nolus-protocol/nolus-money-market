use serde::{Deserialize, Serialize};

use sdk::cosmwasm_std::Uint64;

use crate::{
    contract::{CodeId, Validator},
    result::Result,
};

/// A code id as it arrives on an external, user- or script-facing API boundary.
///
/// Serializes to and deserializes from the exact JSON form of [`Uint64`] (a
/// stringified `u64`), so it is a drop-in replacement for a `Uint64` code-id
/// field with no change on the wire. Unlike [`super::Code`], whose only
/// constructor goes through chain validation, this type holds an unvalidated
/// wire value; the sole way to obtain a validated [`super::Code`] from it is
/// [`Self::try_validate`]. Mirrors the boundary discipline of the external
/// coin wrapper in `finance`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(transparent)]
pub struct Code(Uint64);

impl Code {
    /// Confirm against the chain that the code id exists and coerce to a
    /// validated [`super::Code`].
    pub fn try_validate<V>(self, validator: &V) -> Result<super::Code>
    where
        V: Validator,
    {
        super::Code::try_new(self.0.u64(), validator)
    }
}

impl From<CodeId> for Code {
    fn from(id: CodeId) -> Self {
        Self(Uint64::new(id))
    }
}

#[cfg(test)]
mod tests {
    use sdk::cosmwasm_std::{self, QuerierWrapper, Uint64, testing::MockQuerier};

    use crate::contract::{CodeId, validator};

    use super::Code;

    const ID: CodeId = 42;

    #[test]
    fn wire_form_is_uint64_string() {
        assert_eq!(
            cosmwasm_std::to_json_string(&Uint64::new(ID)).unwrap(),
            cosmwasm_std::to_json_string(&Code::from(ID)).unwrap(),
        );
        assert_eq!(
            "\"42\"",
            cosmwasm_std::to_json_string(&Code::from(ID)).unwrap(),
        );
    }

    #[test]
    fn deserializes_from_the_uint64_string_form() {
        assert_eq!(
            Code::from(ID),
            cosmwasm_std::from_json::<Code>("\"42\"").unwrap(),
        );
    }

    #[test]
    fn try_validate_rejects_an_unknown_code() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);
        assert!(Code::from(ID).try_validate(&validator(querier)).is_err());
    }
}
