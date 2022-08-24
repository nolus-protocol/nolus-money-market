pub enum Either<T1, T2> {
    Left(T1),
    Right(T2),
}

impl<T1, T2> Either<T1, T2> {
    pub fn convert<O>(self) -> O
    where
        T1: Into<O>,
        T2: Into<O>,
    {
        match self {
            Either::Left(value) => value.into(),
            Either::Right(value) => value.into(),
        }
    }
}
