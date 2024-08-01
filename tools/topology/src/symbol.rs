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

        // "ibc/" + 64 hexadecimal digits
        let mut symbol = String::new();

        symbol.reserve_exact(68);

        symbol.push_str("ibc/");

        Sha256::digest(&self.path)
            .into_iter()
            .flat_map(|byte| [byte >> 4, byte & 0xF])
            .map(|half_byte| {
                if half_byte < 10 {
                    b'0' + half_byte
                } else {
                    b'A' + (half_byte - 10)
                }
            })
            .map(char::from)
            .for_each(|ch| symbol.push(ch));

        Symbol {
            path: self.path.into_boxed_str(),
            symbol: symbol.into_boxed_str(),
        }
    }
}
