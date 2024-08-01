use sha2::{Digest as _, Sha256};

#[derive(Debug, PartialEq, Eq)]
pub struct Symbol {
    path: Box<str>,
    symbol: Box<str>,
}

impl Symbol {
    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }
}

#[derive(Debug)]
pub(crate) struct Builder(Inner);

impl Builder {
    // TODO replace with following after upgrade to Rust 1.79+:
    //  ```
    //  pub const fn new() -> Self {
    //      const { Self(Inner::Native) }
    //  }
    pub const NEW: Self = Self(Inner::Native);

    pub fn add_channel(&mut self, channel: &str) {
        match &mut self.0 {
            Inner::Native => self.0 = Inner::Ibc(Ibc::new(channel)),
            Inner::Ibc(ibc_symbol) => ibc_symbol.add_channel(channel),
        }
    }

    pub fn add_symbol(self, symbol: &str) -> Symbol {
        match self.0 {
            Inner::Native => Symbol {
                path: symbol.into(),
                symbol: symbol.into(),
            },
            Inner::Ibc(ibc_symbol) => ibc_symbol.add_symbol(symbol),
        }
    }
}

#[derive(Debug)]
enum Inner {
    Native,
    Ibc(Ibc),
}

#[derive(Debug)]
struct Ibc {
    path: String,
}

impl Ibc {
    const SHA2_256_OUTPUT_SIZE: usize = 32;

    const BYTE_HALVES_PER_CHAR: usize = 2;

    const SYMBOL_PREFIX: &'static str = "ibc/";

    const OUTPUT_SYMBOL_LENGTH: usize =
        Self::SYMBOL_PREFIX.len() + (Self::SHA2_256_OUTPUT_SIZE * Self::BYTE_HALVES_PER_CHAR);

    fn new(outmost_channel: &str) -> Self {
        let mut instance = Self {
            path: String::new(),
        };

        instance.add_channel(outmost_channel);

        instance
    }

    fn add_channel(&mut self, channel: &str) {
        self.path.push_str("transfer/");

        self.path.push_str(channel);

        self.path.push('/');
    }

    fn add_symbol(mut self, symbol: &str) -> Symbol {
        self.path.push_str(symbol);

        let symbol = Self::digest_symbol(&self.path);

        Symbol {
            path: self.path.into_boxed_str(),
            symbol,
        }
    }

    fn digest_symbol(path: &str) -> Box<str> {
        let mut symbol = String::new();

        symbol.reserve_exact(Self::OUTPUT_SYMBOL_LENGTH);

        symbol.push_str(Self::SYMBOL_PREFIX);

        let digest: [u8; Self::SHA2_256_OUTPUT_SIZE] = Sha256::digest(&path).into();

        Self::map_into_hex_iter(digest).for_each(|ch| symbol.push(ch));

        symbol.into()
    }

    fn map_into_hex_iter<const N: usize>(digest: [u8; N]) -> impl Iterator<Item = char> {
        Self::map_into_byte_halves(digest)
            .into_iter()
            .flat_map(|ByteHalves { low, high }| [high, low])
            .map(|half_byte| {
                if half_byte < 10 {
                    b'0' + half_byte
                } else {
                    b'A' + (half_byte - 10)
                }
                .into()
            })
    }

    fn map_into_byte_halves<const N: usize>(digest: [u8; N]) -> [ByteHalves; N] {
        digest.map(|byte| ByteHalves {
            high: byte >> 4,
            low: byte & 0xF,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ByteHalves {
    low: u8,
    high: u8,
}

const _: () = {
    let 68 = Ibc::OUTPUT_SYMBOL_LENGTH else {
        panic!("Output symbol length didn't match well-known value!")
    };
};
