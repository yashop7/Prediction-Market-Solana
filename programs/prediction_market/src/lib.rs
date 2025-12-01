use anchor_lang::prelude::*;
pub mod state;
pub mod instructions;
pub mod error;

use instructions::*;
use state::*;
use error::PredictionMarketError;


declare_id!("BnhQSbprbPZoruJ2WG6YwBDGNgjLg2DhcsHKvwwFa16P");

#[program]
pub mod prediction_market {
    use super::*;

    pub fn initialize_market(
        ctx: Context<InitializeMarket>,
        market_id: u32,
        settlement_deadline : i64
    ) -> Result<()> {
        let market = &mut ctx.accounts.market;

        require!(
            settlement_deadline > Clock::get()?.unix_timestamp,
            PredictionMarketError::InvalidSettlementDeadline
        );
        market.authority = ctx.accounts.authority.key();
        market.market_id = market_id;
        market.settlement_deadline = settlement_deadline;
        market.collateral_mint = ctx.accounts.collateral_mint.key();
        market.collateral_vault = ctx.accounts.collateral_vault.key();
        market.outcome_a_mint = ctx.accounts.outcome_a_mint.key();
        market.outcome_b_mint = ctx.accounts.outcome_b_mint.key();
        market.is_settled = false;
        market.winning_outcome = None;
        market.total_collateral_locked = 0;
        market.bump = ctx.bumps.market;

        msg!("Market initialized: {}", market.market_id);
        Ok(())
    }

    pub fn split_tokens(
        ctx: Context<SplitToken>,
        market_id: u32,
        amount: u64,
    ) -> Result<()> {
        require!(amount > 0 , PredictionMarketError::InvalidAmount);
        
        
        Ok(())
    }
}
