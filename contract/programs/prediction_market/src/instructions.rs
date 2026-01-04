use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Mint};
use crate::state::{Market, OrderBook};
use crate::constants::*;



#[derive(Accounts)]
#[instruction(market_id: u32)]
pub struct InitializeMarket<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + Market::INIT_SPACE,
        seeds = [MARKET_SEED, market_id.to_le_bytes().as_ref()],
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
        seeds = [VAULT_SEED, market_id.to_le_bytes().as_ref()],
        bump
    )]
    pub collateral_vault: Account<'info, TokenAccount>,
    #[account(
        init,
        payer = authority,
        mint::decimals = 6,
        mint::authority = market,
        seeds = [OUTCOME_YES_SEED, market_id.to_le_bytes().as_ref()],
        bump
    )]
    pub outcome_yes_mint: Account<'info, Mint>,
    #[account(
        init,
        payer = authority,
        mint::decimals = 6,
        mint::authority = market,
        seeds = [OUTCOME_NO_SEED, market_id.to_le_bytes().as_ref()],
        bump
    )]
    pub outcome_no_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = authority,
        seeds = [ORDERBOOK_SEED, market_id.to_le_bytes().as_ref()],
        space = OrderBook::space(0), // Start with 0 orders, will realloc as needed
        bump
    )]
    pub orderbook : Account<'info, OrderBook>,


    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

// IN THIS ACCOUNT WE WILL NEED
/**
 * user_outcome_yes => Token Account A of user
 * user_outcome_no => Token Account B of user
 * outcome_yes_mint
 * outcome_no_mint
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
        seeds = [MARKET_SEED, market.market_id.to_le_bytes().as_ref()],
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
        constraint = collateral_vault.key() == market.collateral_vault // We can also used the .owner of vault to verify it's authority of market
    )]
    pub collateral_vault : Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = outcome_yes_mint.key() == market.outcome_yes_mint
    )]
    pub outcome_yes_mint : Account<'info,Mint>,
    #[account(
        mut,
        constraint = outcome_no_mint.key() == market.outcome_no_mint
    )]
    pub outcome_no_mint : Account<'info,Mint>,
    #[account(
        mut,
        constraint = user_outcome_yes.owner == user.key(),
        constraint = user_outcome_yes.mint == market.outcome_yes_mint
    )]
    pub user_outcome_yes : Account<'info, TokenAccount>, // Ohh we willn't make this account here,
    // we will just check it here , Like is it legit or not
    #[account(
        mut,
        constraint = user_outcome_no.owner == user.key(),
        constraint = user_outcome_no.mint == market.outcome_no_mint
    )]
    pub user_outcome_no : Account<'info, TokenAccount>,
    pub token_program : Program<'info,Token>,
}

#[derive(Accounts)]
#[instruction(market_id: u32)]
pub struct MergeTokens<'info>{
    #[account(
        mut,
        seeds = [MARKET_SEED, market.market_id.to_le_bytes().as_ref()],
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
        constraint = collateral_vault.key() == market.collateral_vault // We can also used the .owner of vault to verify it's authority of market
    )]
    pub collateral_vault : Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = outcome_yes_mint.key() == market.outcome_yes_mint
    )]
    pub outcome_yes_mint : Account<'info,Mint>,

    #[account(
        mut,
        constraint = outcome_no_mint.key() == market.outcome_no_mint
    )]
    pub outcome_no_mint : Account<'info,Mint>,

    #[account(
        mut,
        constraint = user_outcome_yes.owner == user.key(),
        constraint = user_outcome_yes.mint == market.outcome_yes_mint
    )]
    pub user_outcome_yes : Account<'info, TokenAccount>, // Ohh we willn't make this account here,
    // we will just check it here , Like is it legit or not

    #[account(
        mut,
        constraint = user_outcome_no.owner == user.key(),
        constraint = user_outcome_no.mint == market.outcome_no_mint
    )]
    pub user_outcome_no : Account<'info, TokenAccount>,
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
        seeds = [MARKET_SEED, market.market_id.to_le_bytes().as_ref()],
        bump = market.bump,
        constraint = market.market_id == market_id
    )]
    pub market : Account<'info,Market>,

        #[account(
        mut,
        constraint = outcome_yes_mint.key() == market.outcome_yes_mint
    )]
    pub outcome_yes_mint : Account<'info,Mint>,

    #[account(
        mut,
        constraint = outcome_no_mint.key() == market.outcome_no_mint
    )]
    pub outcome_no_mint : Account<'info,Mint>,
    pub token_program : Program<'info,Token>
    
}



#[derive(Accounts)]
#[instruction(market_id:u32)]
pub struct ClaimRewards <'info>{
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        seeds = [MARKET_SEED, market.market_id.to_le_bytes().as_ref()],
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
        constraint = collateral_vault.key() == market.collateral_vault // We can also used the .owner of vault to verify it's authority of market
    )]
    pub collateral_vault: Account<'info, TokenAccount>,
     
    #[account(
        mut,
        constraint = outcome_yes_mint.key() == market.outcome_yes_mint
    )]
    pub outcome_yes_mint: Account<'info, Mint>,
    
    #[account(
        mut,
        constraint = outcome_no_mint.key() == market.outcome_no_mint
    )]
    pub outcome_no_mint: Account<'info, Mint>,
    
    #[account(
        mut,
        constraint = user_outcome_yes.mint == market.outcome_yes_mint,
        constraint = user_outcome_yes.owner == user.key()
    )]
    pub user_outcome_yes: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        constraint = user_outcome_no.mint == market.outcome_no_mint,
        constraint = user_outcome_no.owner == user.key()
    )]
    pub user_outcome_no: Account<'info, TokenAccount>,
    
    pub token_program: Program<'info, Token>
}