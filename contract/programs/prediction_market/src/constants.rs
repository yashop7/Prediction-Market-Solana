use anchor_lang::prelude::*;

pub const MARKET_SEED: &[u8] = b"market";
pub const VAULT_SEED: &[u8] = b"vault";
pub const OUTCOME_YES_SEED: &[u8] = b"outcome_a";
pub const OUTCOME_NO_SEED: &[u8] = b"outcome_b"; 
pub const ORDERBOOK_SEED: &[u8] = b"orderbook";
pub const USER_STATS_SEED: &[u8] = b"user_stats";
pub const ESCROW_SEED: &[u8] = b"escrow";
pub const MAX_ORDERBOOK_LENGTH: u32 = 1000; // Can grow up to 1000 orders per side via realloc
pub const INITIAL_ORDERBOOK_CAPACITY: usize = 10; // Start small, grow as needed
pub const MAX_ORDERS_PER_SIDE : usize = 100;
// 1 YES/NO TOKEN = 6 DECIMALS
// 1 COLLATERAL_TOKEN = 1 YES/NO TOKEN