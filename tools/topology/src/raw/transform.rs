use crate::str::container;

pub(super) trait Transform<Container>: Sized
where
    Container: container::Container,
{
    type Output;

    fn transform(self, deduplication_ctx: &mut Container::DeduplicationContext) -> Self::Output;
}
