use anchor_lang::prelude::*;
use anchor_spl::token::{
    self, spl_token::instruction::AuthorityType, Burn, Mint, MintTo, SetAuthority, Token,
    TokenAccount, Transfer,
};
pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use error::PredictionMarketError;
use instructions::*;
use state::*;

declare_id!("BnhQSbprbPZoruJ2WG6YwBDGNgjLg2DhcsHKvwwFa16P");

#[program]
pub mod prediction_market {
    use anchor_spl::token;

    use crate::constants::MAX_ORDERS_PER_SIDE;

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
        market.yes_escrow = ctx.accounts.yes_escrow.key();
        market.no_escrow = ctx.accounts.no_escrow.key();
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

        //Minting Outcome Yes tokens
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

        //Minting Outcome No tokens
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

        let user_stats = &mut ctx.accounts.user_stats_account;

        if user_stats.user == Pubkey::default() {
            user_stats.user = ctx.accounts.user.key();
            user_stats.market_id = market_id;
            user_stats.locked_yes = 0;
            // user_stats.free_yes = 0;
            user_stats.claimable_yes = 0;

            user_stats.locked_no = 0;
            // user_stats.free_no = 0;
            user_stats.claimable_no = 0;

            user_stats.locked_collateral = 0;
            // user_stats.free_collateral = 0;
            user_stats.claimable_collateral = 0;

            user_stats.bump = ctx.bumps.user_stats_account;
        }

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

        // Transfering Collateral Back to user collateral Account
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

        // Now we are revoking the Authorities from the market to mint more Yes/No Tokens

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

    /// Place an order to buy or sell outcome tokens
    /// 
    /// Flow:
    /// - SELL order: Seller's YES/NO tokens locked in escrow immediately
    /// - BUY order: Buyer's collateral locked in vault immediately
    /// - When matched: 
    ///   * Buyer's claimable_yes incremented in their UserStats (can claim later)
    ///   * Seller will withdraw collateral from vault separately
    pub fn place_order(
        ctx: Context<PlaceOrder>,
        side: OrderSide,
        token_type: TokenType,
        quantity: u64,
        price: u64,
    ) -> Result<()> {
        let market = &mut ctx.accounts.market;
        let orderbook = &mut ctx.accounts.orderbook;

        require!(
            Clock::get()?.unix_timestamp < market.settlement_deadline,
            PredictionMarketError::MarketExpired
        );

        require!(
            !market.is_settled,
            PredictionMarketError::MarketAlreadySettled
        );

        require!(quantity > 0, PredictionMarketError::InvalidOrderQuantity);
        // There should be another checks for Lamports, We can't pay less than the minimum decimals of the Token
        require!(price > 0, PredictionMarketError::InvalidOrderPrice);

        // Now I will Swap Assests
        // taking yes/no tokens from the User Account and Transfer into the escrow account

        let (mint_type, user_token_account, token_escrow) = match token_type {
            TokenType::Yes => (
                market.outcome_yes_mint,
                &ctx.accounts.user_outcome_yes,
                &ctx.accounts.yes_escrow,
            ),
            TokenType::No => (
                market.outcome_no_mint,
                &ctx.accounts.user_outcome_no,
                &ctx.accounts.no_escrow,
            ),
        };

        let amount = quantity
            .checked_mul(price)
            .ok_or(PredictionMarketError::MathOverflow)?;

        //Checking if user has enough balance or not

        // Lock funds immediately when placing order

        if side == OrderSide::Sell {
            // Seller: Lock YES/NO tokens in escrow
            require!(
                user_token_account.amount >= quantity,
                PredictionMarketError::NotEnoughBalance
            );

            token::transfer(
                CpiContext::new(
                    ctx.accounts.token_program.to_account_info(),
                    Transfer {
                        from: user_token_account.to_account_info(),
                        to: token_escrow.to_account_info(),
                        authority: ctx.accounts.user.to_account_info(),
                    },
                ),
                quantity,
            )?;
        } else {
            // Buyer: Lock collateral in vault
            require!(
                ctx.accounts.user_collateral.amount >= amount,
                PredictionMarketError::NotEnoughBalance
            );

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
            )?;
        }

        let mut order = Order {
            id: orderbook.next_order_id,
            market_id: market.market_id,
            user_key: ctx.accounts.user.key(),
            side,
            token_type,
            price,
            quantity,
            filledquantity: 0,
            timestamp: Clock::get()?.unix_timestamp,
        };
        orderbook.next_order_id += 1;

        // Then we will add the orderbook in one of the vector
        // I have to increase the Space according to new Incoming order
        let yes_buy_orders = &mut orderbook.yes_buy_orders;
        let yes_sell_orders = &mut orderbook.yes_sell_orders;
        let no_buy_orders = &mut orderbook.no_buy_orders;
        let no_sell_orders = &mut orderbook.no_sell_orders;

        // let order_vec = match (token_type, side) {
        //     (TokenType::Yes, OrderSide::Buy) => yes_buy_orders,
        //     (TokenType::Yes, OrderSide::Sell) => yes_sell_orders,
        //     (TokenType::No, OrderSide::Buy) => no_buy_orders,
        //     (TokenType::No, OrderSide::Sell) => no_sell_orders,
        // };

        // if (order_vec.len() > MAX_ORDERS_PER_SIDE) {
        //     return Err(PredictionMarketError::MaxOrdersReached.into());
        // }

        // order_vec.push(order);

        // Now we will arrange the order in Inc & Dec Amount

        let mut idx = 0;
        let mut iteration = 0;
        let mut max_iteration = 0;
        let mut completed_orders: Vec<Order> = Vec::new();

        if token_type == TokenType::Yes {
            if order.side == OrderSide::Buy {
                while idx < yes_sell_orders.len() && iteration <= max_iteration {
                    // I will iterate in this Array
                    let (book_price, book_qty, book_filled_qty) = {
                        let order = &yes_sell_orders[idx];
                        (order.price, order.quantity, order.filledquantity)
                    };
                    // Sell order are in the Arrangement [(price,quantity)]
                    if (order.price >= book_price) {
                        // Checking how much quantity is remaining to both of user
                        let buyer_left_qty = order
                            .quantity
                            .checked_sub(order.filledquantity)
                            .ok_or(PredictionMarketError::MathOverflow)?;
                        let seller_left_qty = book_qty
                            .checked_sub(book_filled_qty)
                            .ok_or(PredictionMarketError::MathOverflow)?;
                        let min_qty = buyer_left_qty.min(seller_left_qty);

                        // Now we will decrease the min_qty
                        yes_sell_orders[idx].filledquantity = book_filled_qty
                            .checked_add(min_qty)
                            .ok_or(PredictionMarketError::MathOverflow)?;

                        order.filledquantity = order
                            .filledquantity
                            .checked_add(min_qty)
                            .ok_or(PredictionMarketError::MathOverflow)?;

                        // Execute the trade:
                        // 1. Transfer YES tokens: escrow → buyer (current user)
                        // 2. Transfer collateral: vault → seller (from matched order)

                        let market_id_bytes = market.market_id.to_le_bytes();
                        let seeds = &[b"market", market_id_bytes.as_ref(), &[market.bump]];

                        let collateral_amount = min_qty
                            .checked_mul(book_price)
                            .ok_or(PredictionMarketError::MathOverflow)?;

                        // Credit buyer's claimable YES tokens
                        ctx.accounts.user_stats_account.claimable_yes = ctx.accounts.user_stats_account
                            .claimable_yes
                            .checked_add(min_qty)
                            .ok_or(PredictionMarketError::MathOverflow)?;

                        // Credit seller's claimable collateral
                        let seller_pubkey = yes_sell_orders[idx].user_key;
                        let seller_stats_pda = Pubkey::find_program_address(
                            &[
                                b"user_stats",
                                seller_pubkey.as_ref(),
                                market.market_id.to_le_bytes().as_ref(),
                            ],
                            ctx.program_id,
                        ).0;

                        // Find seller's UserStats in remaining_accounts and update claimable_collateral
                        let mut seller_credited = false;
                        for account_info in ctx.remaining_accounts.iter() {
                            if account_info.key == &seller_stats_pda {
                                let mut data = account_info.try_borrow_mut_data()?;
                                let mut seller_stats = UserStats::try_deserialize(&mut &data[..])?;
                                
                                seller_stats.claimable_collateral = seller_stats
                                    .claimable_collateral
                                    .checked_add(collateral_amount)
                                    .ok_or(PredictionMarketError::MathOverflow)?;
                                
                                let mut writer = &mut data[..];
                                seller_stats.try_serialize(&mut writer)?;
                                
                                seller_credited = true;
                                break;
                            }
                        }

                        require!(
                            seller_credited,
                            PredictionMarketError::SellerStatsAccountNotProvided
                        );

                        msg!(
                            "Trade executed: Buyer +{} claimable YES, Seller +{} claimable collateral",
                            min_qty,
                            collateral_amount
                        );

                        // Mark completed orders for removal
                        if yes_sell_orders[idx].filledquantity == yes_sell_orders[idx].quantity {
                            completed_orders.push(yes_sell_orders[idx].clone());
                        }

                        //We will have to add the order in the Completed order, like where it is belong to

                        // Who is minimum among the filled quantity & the amount, we will confirm it
                        // Now we will decrease the filled qty from the person & we will give the assets from the collateral
                        // Then we will have to give user the user Tokens from the Yes Escrow
                        // Then there is also a Task to remove the order from the book whose Filled Qty are equal to the quantity
                        // Then there is also a chance that orders are not complete// then we will have to put the order in the orderbook, If anthing is not matched
                        // There are 4 cases
                        // His order completed at the same time
                        // His order completed at the same time,(but iteration reached their max) But the order is too big, then it's failing, So then we will either have to use Crank Bot, How to solve this, To Lazily complete the order, WE WILL HAVE TO SEE MORE ON THIS
                        // His order is Partially filled & rest is added to the orderbook
                        // His order is ILLOGICAL, so we add the order to the orderbook right away

                        // there is a Buy Order

                        // In that case my money is debited from my account -> Collateral Vault,
                        // then collateralvault -> Other user Wallet,
                        // Escrow has yes tokens of that person, yes Escrow => My Yes token account

                        // In Case of Sell orders

                        // I have already given my tokens to the escrow
                        // What If suddenly order match ?
                        // Other user yes Escrow => His yes token Account
                        // I will get his collateral; CollateralVault => Collateral Account

                        // What If not Instant order Succes? Then If anbody comes to Buy
                        // My order is in Orderbook, My Yes tokens are in escrow
                        // If order matches later
                        // Me=> Collateral Vault => My collateral Account
                        // Other user => Yes escrow => other user escrow Account

                        iteration += 1;
                    }

                    idx += 1;
                }
            } else {
            }
        } else {
            if side == OrderSide::Buy {
            } else {
            }
        }

        // We will also have to remove the order whose filled quantity == quantity

        // At the end we will arrange all the order in the Chronological Order

        Ok(())
    }
}
