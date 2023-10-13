use crate::error::ContractError;

pub type ContractResult<T> = Result<T, ContractError>;
