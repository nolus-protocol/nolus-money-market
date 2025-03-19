use crate::connection::ConnectionParams;

pub trait Connectable {
    fn dex(&self) -> &ConnectionParams;
}
