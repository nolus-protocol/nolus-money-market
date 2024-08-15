use sdk::schemars;

use crate::{define_currency, Lpns};

define_currency!(
    UsdcNoble,
    "ibc/83C68CA1189A7DAC4FDA8B89F9166FFA6BA3A8C5534B0E3D84D831B4F350FE59", // transfer/channel-0/transfer/channel-4280/uusdc
    "ibc/DE6792CF9E521F6AD6E9A4BDF6225C9571A3B74ACC0A529F92BC5122A39D2E58", // transfer/channel-4280/uusdc
    Lpns,
    6
);

pub use UsdcNoble as Lpn;
