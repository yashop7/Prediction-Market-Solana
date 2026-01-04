use anchor_lang::prelude::*;
use anchor_spl::token::{
    self, spl_token::instruction::AuthorityType, Burn, Mint, MintTo, SetAuthority, Token,
    TokenAccount, Transfer,
};
pub mod error;
pub mod instructions;
pub mod state;
pub mod constants;

use error::PredictionMarketError;
use instructions::*;
use state::*;

declare_id!("BnhQSbprbPZoruJ2WG6YwBDGNgjLg2DhcsHKvwwFa16P");

#[program]
pub mod prediction_market {
    use anchor_spl::token;

    use super::*;

    pub fn initialize_market(
        ctx: Context<InitializeMarket>,
        market_id: u32,
        settlement_deadline: i64,
    ) -> Result<()> {
        let market: &mut Account<'_, Market> = &mut ctx.accounts.market;
        require!(
            settlement_deadline > Clock::get()?.unix_timestamp,
            PredictionMarketError::InvalidSettlementDeadline
        );
        market.authority = ctx.accounts.authority.key();
        market.market_id = market_id;
        market.settlement_deadline = settlement_deadline;
        market.collateral_mint = ctx.accounts.collateral_mint.key();
        market.collateral_vault = ctx.accounts.collateral_vault.key();
        market.outcome_yes_mint = ctx.accounts.outcome_yes_mint.key();
        market.outcome_no_mint = ctx.accounts.outcome_no_mint.key();
        market.is_settled = false;
        market.winning_outcome = None;
        market.total_collateral_locked = 0;
        market.bump = ctx.bumps.market;

        let orderbook = &mut ctx.accounts.orderbook;
        orderbook.bump = ctx.bumps.orderbook;
        orderbook.market_id = market_id;
        orderbook.next_order_id = 0;
        orderbook.yes_buy_orders = Vec::new();
        orderbook.yes_sell_orders = Vec::new();
        orderbook.no_buy_orders = Vec::new();
        orderbook.no_sell_orders = Vec::new();


        msg!("Market initialized: {}", market.market_id);
        Ok(())
    }

    pub fn split_tokens(ctx: Context<SplitToken>, market_id: u32, amount: u64) -> Result<()> {
        let market = &mut ctx.accounts.market;
        require!(amount > 0, PredictionMarketError::InvalidAmount);
        require!(
            !market.is_settled,
            PredictionMarketError::MarketAlreadySettled
        );
        require!(
            Clock::get()?.unix_timestamp < market.settlement_deadline,
            PredictionMarketError::MarketExpired
        );

        //Difference
        //         ctx.accounts.user_collateral         // Type: Account<TokenAccount>
        //         ctx.accounts.user_collateral.to_account_info()  // Type: AccountInfo

        // Transferring the tokens from user account into Collateral Vault
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user_collateral.to_account_info(),
                    to: ctx.accounts.collateral_vault.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            amount,
        );
        let market_id_bytes = market.market_id.to_le_bytes();
        let seeds = &[b"market", market_id_bytes.as_ref(), &[market.bump]];

        //Minting Outcome A tokens
        token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.outcome_yes_mint.to_account_info(),
                    to: ctx.accounts.user_outcome_yes.to_account_info(),
                    authority: market.to_account_info(),
                },
                &[seeds],
            ),
            amount,
        );

        //Minting Outcome B tokens
        token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.outcome_no_mint.to_account_info(),
                    to: ctx.accounts.user_outcome_no.to_account_info(),
                    authority: market.to_account_info(),
                },
                &[seeds],
            ),
            amount,
        );

        market.total_collateral_locked = market
            .total_collateral_locked
            .checked_add(amount)
            .ok_or(PredictionMarketError::MathOverflow)?;
        // What ok_or is doing that, we are getting the Some(value) or None from the checked_add
        // ok_or is returning us Result<u64,Err>,
        // .ok_or(error) → converts to Result<u64, Error>:
        // Some(value) → Ok(value)
        // None → Err(PredictionMarketError::MathOverflow)
        msg!("Minted {} outcome tokens for user", amount);
        Ok(())
    }

    pub fn merge_tokens(ctx: Context<MergeTokens>, market_id: u32) -> Result<()> {
        let market = &mut ctx.accounts.market;

        require!(
            Clock::get()?.unix_timestamp < market.settlement_deadline,
            PredictionMarketError::MarketExpired
        );
        require!(
            !market.is_settled,
            PredictionMarketError::MarketAlreadySettled
        );

        let balA = ctx.accounts.user_outcome_yes.amount;
        let balB = ctx.accounts.user_outcome_no.amount;

        let amount = balA.min(balB);

        require!(amount > 0, PredictionMarketError::InvalidAmount);

        token::burn(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Burn {
                    mint: ctx.accounts.outcome_yes_mint.to_account_info(),
                    from: ctx.accounts.user_outcome_yes.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            amount,
        );
        token::burn(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Burn {
                    mint: ctx.accounts.outcome_no_mint.to_account_info(),
                    from: ctx.accounts.user_outcome_no.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            amount,
        );

        let market_id_bytes = market.market_id.to_le_bytes();
        let seeds = &[b"market", market_id_bytes.as_ref(), &[market.bump]];

        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.collateral_vault.to_account_info(),
                    to: ctx.accounts.user_collateral.to_account_info(),
                    authority: market.to_account_info(),
                },
                &[seeds],
            ),
            amount,
        );

        market.total_collateral_locked = market
            .total_collateral_locked
            .checked_sub(amount)
            .ok_or(PredictionMarketError::MathOverflow)?;

        msg!(
            "Merged {} pairs of outcome tokens back to collateral",
            amount
        );
        Ok(())
    }

    pub fn set_winning_side(
        ctx: Context<SetWinner>,
        market_id: u32,
        winning_outcome: WinningOutcome,
    ) -> Result<()> {
        let market = &mut ctx.accounts.market;

        require!(
            Clock::get()?.unix_timestamp < market.settlement_deadline,
            PredictionMarketError::MarketExpired
        );
        require!(
            !market.is_settled,
            PredictionMarketError::MarketAlreadySettled
        );

        market.is_settled = true;
        // Setting the Winning Outcome
        market.winning_outcome = Some(winning_outcome);

        // Now we are revoking the Authorities from the market to mint more Tokens A or B

        let market_id_bytes = market.market_id.to_le_bytes();
        let seeds = &[b"market", market_id_bytes.as_ref(), &[market.bump]];

        token::set_authority(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                SetAuthority {
                    current_authority: market.to_account_info(),
                    account_or_mint: ctx.accounts.outcome_yes_mint.to_account_info(),
                },
                &[seeds],
            ),
            AuthorityType::MintTokens,
            None,
        )?;

        token::set_authority(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                SetAuthority {
                    current_authority: market.to_account_info(),
                    account_or_mint: ctx.accounts.outcome_no_mint.to_account_info(),
                },
                &[seeds],
            ),
            AuthorityType::MintTokens,
            None,
        )?;

        msg!("Wining Outcome is Set to be: {:?}", winning_outcome);

        Ok(())
    }

    pub fn claim_rewards(ctx: Context<ClaimRewards>, market_id: u32) -> Result<()> {
        let market: &mut Account<'_, Market> = &mut ctx.accounts.market;

        require!(market.is_settled, PredictionMarketError::MarketNotSettled);

        let winner = market
            .winning_outcome
            .ok_or_else(|| PredictionMarketError::WinningOutcomeNotSet)?;

        let (winner_user_ata, winner_mint) = match winner {
            WinningOutcome::OutcomeA => (
                &ctx.accounts.user_outcome_yes,
                ctx.accounts.outcome_yes_mint.to_account_info(),
            ),
            _ => (
                &ctx.accounts.user_outcome_no,
                ctx.accounts.outcome_no_mint.to_account_info(),
            ),
        };

        // now we will burn the Tokens of Other user

        let amount = winner_user_ata.amount;

        // Burning Winnning Tokens
        token::burn(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Burn {
                    mint: winner_mint,
                    from: winner_user_ata.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            amount,
        )?;

        // Now we will transfer collateral tokens from the vault to the user

        let market_id_bytes = market.market_id.to_le_bytes();
        let signer = &[b"market", market_id_bytes.as_ref(), &[market.bump]];

        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.collateral_vault.to_account_info(),
                    to: ctx.accounts.user_collateral.to_account_info(),
                    authority: market.to_account_info(),
                },
                &[signer],
            ),
            amount,
        )?;

        market.total_collateral_locked = market
            .total_collateral_locked
            .checked_sub(amount)
            .ok_or(PredictionMarketError::MathOverflow)?;

        msg!("Claimed Awards by user {}", amount);

        Ok(())
    }

   
}
