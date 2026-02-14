use borsh::{BorshDeserialize, BorshSerialize};

#[derive(Clone, Debug, Eq, Hash, PartialEq, BorshSerialize, BorshDeserialize, Copy)]
pub struct Address([u8; 32]);

impl Address {
    pub const SYSTEM: Address = Address([0u8; 32]);

    /// Creates an Address from a 32-byte array.
    pub fn new(bytes: [u8; 32]) -> Self {
        Address(bytes)
    }

    /// Creates an Address from bytes (must be exactly 32 bytes).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() != 32 {
            return Err("Address must be exactly 32 bytes");
        }
        let mut addr_bytes = [0u8; 32];
        addr_bytes.copy_from_slice(bytes);
        Ok(Address(addr_bytes))
    }

    /// Returns the address as a 32-byte array.
    pub fn as_bytes(&self) -> [u8; 32] {
        self.0
    }
}
