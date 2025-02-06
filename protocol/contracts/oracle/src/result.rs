use crate::error::Error;

pub type Result<T, ErrorG> = std::result::Result<T, Error<ErrorG>>;
