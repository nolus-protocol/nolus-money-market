pub(crate) trait Pipe: Sized {
    fn pipe_mut<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut Self),
    {
        f(&mut self);

        self
    }

    fn pipe_if<F>(self, value: bool, f: F) -> Self
    where
        F: FnOnce(Self) -> Self,
    {
        if value { f(self) } else { self }
    }

    fn pipe_if_some<T, F>(self, value: Option<T>, f: F) -> Self
    where
        F: FnOnce(Self, T) -> Self,
    {
        if let Some(value) = value {
            f(self, value)
        } else {
            self
        }
    }
}

impl<T> Pipe for T {}
