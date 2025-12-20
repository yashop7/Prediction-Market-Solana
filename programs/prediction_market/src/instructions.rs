use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Mint};
use crate::state::Market;



#[derive(Accounts)]
#[instruction(market_id: u32)]
pub struct InitializeMarket<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + Market::INIT_SPACE,
        seeds = [b"market", market_id.to_le_bytes().as_ref()],
        bump
    )]
    pub market: Account<'info, Market>,
    
    #[account(mut)]
    pub authority: Signer<'info>,

    pub collateral_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = authority,
        token::mint = collateral_mint,
        token::authority = market,
        seeds = [b"vault",market_id.to_le_bytes().as_ref()],
        bump
    )]
    pub collateral_vault: Account<'info, TokenAccount>,
    #[account(
        init,
        payer = authority,
        mint::decimals = 6,
        mint::authority = market,
        seeds = [b"outcome_a",market_id.to_le_bytes().as_ref()],
        bump
    )]
    pub outcome_a_mint: Account<'info, Mint>,
    #[account(
        init,
        payer = authority,
        mint::decimals = 6,
        mint::authority = market,
        seeds = [b"outcome_b",market_id.to_le_bytes().as_ref()],
        bump
    )]
    pub outcome_b_mint: Account<'info, Mint>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

// IN THIS ACCOUNT WE WILL NEED
/**
 * user_outcome_a => Token Account A of user
 * user_outcome_b => Token Account B of user
 * outcome_a_mint
 * outcome_b_mint
 * Collateral_mint ==> No need we will read this from market
 * Collateral_vault
 * Token program
 * System Program
 * user => Signer, OK this time user is a Signer
 * market => which market is this, This time we will check the market,
 * that If it really exist and we also have market_id to double check it
 * 
 */


 // ACCORDING TO ME THIS IS HAPPENING LIKE
 // WE WILL SEND THE MARKET PUBLIC KEY
 // AND ANCHOR FIND'S IT AND WILL TAKE THE INFORMATION FROM THE MARKET ADDRESS
 // THEN IT WILL CHECK ON IT'S OWN THAT 
 // 1. BY PUTTING BUMP FROM THE DATA IT GET'S
 // 2. 2ND BY COMPARING SEEDS FROM THE MARKET_ID WHICH WE REALLY SENDED
 // SO IT DOUBLE CHECKS EVERYTHING

#[derive(Accounts)]
#[instruction(market_id: u32)]
pub struct SplitToken<'info> {
    #[account(
        mut,
        seeds = [b"market", market.market_id.to_le_bytes().as_ref()],
        bump = market.bump,
        constraint = market.market_id == market_id
    )]
    pub market : Account<'info,Market>, // Remember that You will input the market address
    // Anchor loads the Data of that particular Add. and pick the details from it
    // and then verify the information
    // We will also verify that we market_id belong to the market add. or not

    #[account(mut)]
    pub user : Signer<'info>,

    #[account(
        mut,
        constraint = user_collateral.mint == market.collateral_mint,
        constraint = user_collateral.owner == user.key()
    )]
    pub user_collateral : Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = collateral_vault.key() == market.collateral_vault
    )]
    pub collateral_vault : Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = outcome_a_mint.key() == market.outcome_a_mint
    )]
    pub outcome_a_mint : Account<'info,Mint>,
    #[account(
        mut,
        constraint = outcome_b_mint.key() == market.outcome_b_mint
    )]
    pub outcome_b_mint : Account<'info,Mint>,
    #[account(
        mut,
        constraint = user_outcome_a.owner == user.key(),
        constraint = user_outcome_a.mint == market.outcome_a_mint
    )]
    pub user_outcome_a : Account<'info, TokenAccount>, // Ohh we willn't make this account here,
    // we will just check it here , Like is it legit or not
    #[account(
        mut,
        constraint = user_outcome_b.owner == user.key(),
        constraint = user_outcome_b.mint == market.outcome_b_mint
    )]
    pub user_outcome_b : Account<'info, TokenAccount>,
    pub token_program : Program<'info,Token>,
}

#[derive(Accounts)]
#[instruction(market_id: u32)]
pub struct MergeTokens<'info>{
    #[account(
        mut,
        seeds = [b"market", market.market_id.to_le_bytes().as_ref()],
        bump = market.bump,
        constraint = market.market_id == market_id
    )]
    pub market : Account<'info,Market>,

    #[account(mut)]
    pub user : Signer<'info>,

    #[account(
        mut,
        constraint = user_collateral.mint == market.collateral_mint,
        constraint = user_collateral.owner == user.key()
    )]
    pub user_collateral : Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = collateral_vault.key() == market.collateral_vault
    )]
    pub collateral_vault : Account<'info,TokenAccount>,

    #[account(
        mut,
        constraint = outcome_a_mint.key() == market.outcome_a_mint
    )]
    pub outcome_a_mint : Account<'info,Mint>,

    #[account(
        mut,
        constraint = outcome_b_mint.key() == market.outcome_b_mint
    )]
    pub outcome_b_mint : Account<'info,Mint>,

    #[account(
        mut,
        constraint = user_outcome_a.owner == user.key(),
        constraint = user_outcome_a.mint == market.outcome_a_mint
    )]
    pub user_outcome_a : Account<'info, TokenAccount>, // Ohh we willn't make this account here,
    // we will just check it here , Like is it legit or not

    #[account(
        mut,
        constraint = user_outcome_b.owner == user.key(),
        constraint = user_outcome_b.mint == market.outcome_b_mint
    )]
    pub user_outcome_b : Account<'info, TokenAccount>,
    pub token_program : Program<'info,Token>
}


//Like who will be the winner
#[derive(Accounts)]
#[instruction(market_id :u32)]
pub struct SetWinner <'info>{

    #[account(mut)]
    pub authority : Signer<'info>,

    #[account(
        mut,
        seeds = [b"market", market.market_id.to_le_bytes().as_ref()],
        bump = market.bump,
        constraint = market.market_id == market_id
    )]
    pub market : Account<'info,Market>,

        #[account(
        mut,
        constraint = outcome_a_mint.key() == market.outcome_a_mint
    )]
    pub outcome_a_mint : Account<'info,Mint>,

    #[account(
        mut,
        constraint = outcome_b_mint.key() == market.outcome_b_mint
    )]
    pub outcome_b_mint : Account<'info,Mint>,
    pub token_program : Program<'info,Token>
    
}



#[derive(Accounts)]
#[instruction(market_id:u32)]
pub struct ClaimRewards <'info>{
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        seeds = [b"market", market.market_id.to_le_bytes().as_ref()],
        bump = market.bump,
        constraint = market.market_id == market_id
    )]
    pub market: Account<'info, Market>,

    #[account(
        mut,
        constraint = user_collateral.mint == market.collateral_mint,
        constraint = user_collateral.owner == user.key()
    )]
    pub user_collateral: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        constraint = collateral_vault.key() == market.collateral_vault
    )]
    pub collateral_vault: Account<'info, TokenAccount>,
     
    #[account(
        mut,
        constraint = outcome_a_mint.key() == market.outcome_a_mint
    )]
    pub outcome_a_mint: Account<'info, Mint>,
    
    #[account(
        mut,
        constraint = outcome_b_mint.key() == market.outcome_b_mint
    )]
    pub outcome_b_mint: Account<'info, Mint>,
    
    #[account(
        mut,
        constraint = user_outcome_a.mint == market.outcome_a_mint,
        constraint = user_outcome_a.owner == user.key()
    )]
    pub user_outcome_a: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        constraint = user_outcome_b.mint == market.outcome_b_mint,
        constraint = user_outcome_b.owner == user.key()
    )]
    pub user_outcome_b: Account<'info, TokenAccount>,
    
    pub token_program: Program<'info, Token>,
 
}