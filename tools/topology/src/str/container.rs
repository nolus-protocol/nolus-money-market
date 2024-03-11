use std::{borrow::Borrow, fmt::Display, ops::Deref, rc::Rc, sync::Arc};

use super::pool::Pool;

pub trait Container: Deref<Target = str> + Borrow<str> + Display + Ord + Clone + 'static {
    type DeduplicationContext: ?Sized;

    fn new_deduplication_context() -> Self::DeduplicationContext;

    fn new_deduplicated(s: &str, ctx: &mut Self::DeduplicationContext) -> Self;
}

impl Container for String {
    type DeduplicationContext = ();

    fn new_deduplication_context() -> Self::DeduplicationContext {}

    fn new_deduplicated(s: &str, &mut (): &mut Self::DeduplicationContext) -> Self {
        s.into()
    }
}

impl Container for Box<str> {
    type DeduplicationContext = ();

    fn new_deduplication_context() -> Self::DeduplicationContext {}

    fn new_deduplicated(s: &str, &mut (): &mut Self::DeduplicationContext) -> Self {
        s.into()
    }
}

impl Container for Rc<str> {
    type DeduplicationContext = Pool<Self>;

    fn new_deduplication_context() -> Self::DeduplicationContext {
        Pool::new()
    }

    fn new_deduplicated(s: &str, ctx: &mut Self::DeduplicationContext) -> Self {
        ctx.get_or_insert(s)
    }
}

impl Container for Arc<str> {
    type DeduplicationContext = Pool<Self>;

    fn new_deduplication_context() -> Self::DeduplicationContext {
        Pool::new()
    }

    fn new_deduplicated(s: &str, ctx: &mut Self::DeduplicationContext) -> Self {
        ctx.get_or_insert(s)
    }
}
