use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace)]
pub enum WinningOutcome {
    OutcomeA,
    OutcomeB,
    Neither, // Draw or invalid outcome - both tokens get 50% payout
}

#[account]
#[derive(InitSpace)]
pub struct Market {
    pub authority: Pubkey,
    pub market_id: u32, // This will be a No.
    pub settlement_deadline: i64,
    pub outcome_a_mint: Pubkey,
    pub outcome_b_mint: Pubkey,
    pub collateral_mint: Pubkey,// Can be USDC.. etc
    pub collateral_vault: Pubkey,
    pub is_settled: bool,
    pub winning_outcome: Option<WinningOutcome>,
    pub total_collateral_locked: u64,
    pub bump: u8,
    // We can also Put META data URL, which is stored offchain in some S3 storage
    // That Meta URL data consists of market image, Name , kind of like an object 
    // put some max lenght on that metadata_url
}