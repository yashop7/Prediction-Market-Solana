use anchor_lang::prelude::*;

use crate::constants::MAX_ORDERBOOK_LENGTH;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace)]
pub enum WinningOutcome {
    OutcomeA,
    OutcomeB,
    Neither, // Draw or invalid outcome - both tokens get 50% payout
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace)]
pub enum TokenType {
    Yes,
    No
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace)]
pub enum OrderSide {
    Buy,
    Sell
}

#[account]
#[derive(InitSpace)]
pub struct Market {
    pub authority: Pubkey,
    pub market_id: u32, // This will be a No.
    pub settlement_deadline: i64,
    pub collateral_mint: Pubkey,// Can be USDC.. etc
    pub collateral_vault: Pubkey,
    pub is_settled: bool,
    pub winning_outcome: Option<WinningOutcome>,
    pub total_collateral_locked: u64,
    pub bump: u8,
    // We can also Put META data URL, which is stored offchain in some S3 storage
    // That Meta URL data consists of market image, Name , kind of like an object 
    // put some max lenght on that metadata_url
    #[max_len(200)]
    pub meta_data_url : String,
    pub outcome_yes_mint: Pubkey, // Type of mint of YES & NO
    pub outcome_no_mint: Pubkey,
    pub yes_escrow : Pubkey, // Escrow Account to store the Yes/No
    pub no_escrow : Pubkey,
}


#[account]
#[derive(InitSpace)]
pub struct UserStats { // User Account associated with the particular market
    pub user : Pubkey,
    pub market_id : u32,

    // pub total_yes_bought: u64,
    // pub total_yes_sold: u64,
    // pub total_no_bought: u64,
    // pub total_no_sold: u64,

    pub claimable_yes: u64,
    pub locked_yes: u64,
    // pub free_yes: u64,

    pub claimable_no: u64,
    pub locked_no: u64,
    // pub free_no: u64,

    pub claimable_collateral: u64,
    pub locked_collateral: u64,
    // pub free_collateral: u64,

    pub reward_claimed : bool,
    pub bump : u8
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace)]
pub struct Order {
    pub id : u64,
    pub market_id : u32,
    pub user_key : Pubkey,
    pub side : OrderSide,
    pub token_type : TokenType,
    pub price : u64,
    pub quantity : u64,
    pub filledquantity : u64,
    pub timestamp : i64

}

#[account]
pub struct OrderBook {
    pub market_id : u32,
    pub next_order_id : u64,
    pub yes_buy_orders : Vec<Order>,
    pub yes_sell_orders : Vec<Order>,
    pub no_buy_orders : Vec<Order>,
    pub no_sell_orders : Vec<Order>,
    pub bump : u8
}

impl OrderBook {
    // Discriminator (8) + market_id (4) + next_order_id (8) + bump (1) = 21
    // Each Vec has 4 bytes for length prefix = 4 * 4 = 16
    pub const BASE_SIZE: usize = 8 + 4 + 8 + 1 + 16;
    
    // Size of each Order struct
    pub const ORDER_SIZE: usize = 80; // Padded size
    
    // Calculate space needed for N orders per side
    pub fn space(orders_per_side: usize) -> usize {
        Self::BASE_SIZE + (orders_per_side * Self::ORDER_SIZE * 4) // 4 vectors
    }
    
    // Calculate current total orders across all sides
    pub fn total_orders(&self) -> usize {
        self.yes_buy_orders.len() + 
        self.yes_sell_orders.len() + 
        self.no_buy_orders.len() + 
        self.no_sell_orders.len()
    }
    
    // Calculate required space based on current orders
    pub fn current_space_needed(&self) -> usize {
        let max_per_side = self.yes_buy_orders.len()
            .max(self.yes_sell_orders.len())
            .max(self.no_buy_orders.len())
            .max(self.no_sell_orders.len());
        Self::space(max_per_side)
    }
}


