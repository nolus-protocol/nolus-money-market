use crate::connection::ConnectionParams;

pub trait DexConnectable {
    fn dex(&self) -> &ConnectionParams;
}
