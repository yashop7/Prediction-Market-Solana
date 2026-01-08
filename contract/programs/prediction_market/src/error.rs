use anchor_lang::prelude::*;

#[error_code]
pub enum PredictionMarketError {
    #[msg("Invalid settlement deadline")]
    InvalidSettlementDeadline,
    #[msg("Market already settled")]
    MarketAlreadySettled,
    #[msg("Market has expired")]
    MarketExpired,
    #[msg("Invalid amount")]
    InvalidAmount,
    #[msg("Invalid order quantity")]
    InvalidOrderQuantity,
    #[msg("Invalid order price")]
    InvalidOrderPrice,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Invalid winning outcome")]
    InvalidWinningOutcome,
    #[msg("Market is not setteld yet")]
    MarketNotSettled,
    #[msg("Winning outcome is not set yet")]
    WinningOutcomeNotSet,
    #[msg("Max Orders reached for this Side")]
    MaxOrdersReached,
    #[msg("Not enough Balance in the account")]
    NotEnoughBalance,
    #[msg("Seller's UserStats account not provided in remaining_accounts")]
    SellerStatsAccountNotProvided,
}