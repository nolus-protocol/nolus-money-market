pub(crate) trait Transform
where
    Self: Sized,
{
    type Context<'r>: ?Sized;

    type Output;

    type Error;

    fn transform(self, ctx: &Self::Context<'_>) -> Result<Self::Output, Self::Error>;
}
