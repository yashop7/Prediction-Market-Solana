use anchor_lang::prelude::*;

pub const MARKET_SEED: &[u8] = b"market";
pub const VAULT_SEED: &[u8] = b"vault";
pub const OUTCOME_YES_SEED: &[u8] = b"outcome_a";
pub const OUTCOME_NO_SEED: &[u8] = b"outcome_b"; 
pub const ORDERBOOK_SEED: &[u8] = b"orderbook";
pub const MAX_ORDERBOOK_LENGTH: u32 = 1000; // Can grow up to 1000 orders per side via realloc
pub const INITIAL_ORDERBOOK_CAPACITY: usize = 10; // Start small, grow as needed