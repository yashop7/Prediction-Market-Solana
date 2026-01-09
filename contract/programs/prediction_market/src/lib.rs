use anchor_lang::prelude::*;
use anchor_spl::token::{
    spl_token::instruction::AuthorityType, Burn, MintTo, SetAuthority, Transfer,
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
        )?;
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
        )?;

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
        )?;

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

    pub fn merge_tokens(ctx: Context<MergeTokens>, _market_id: u32) -> Result<()> {
        let market = &mut ctx.accounts.market;

        require!(
            Clock::get()?.unix_timestamp < market.settlement_deadline,
            PredictionMarketError::MarketExpired
        );
        require!(
            !market.is_settled,
            PredictionMarketError::MarketAlreadySettled
        );

        let bal_a = ctx.accounts.user_outcome_yes.amount;
        let bal_b = ctx.accounts.user_outcome_no.amount;

        let amount = bal_a.min(bal_b);

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
        )?;
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
        )?;

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
        )?;

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
        _market_id: u32,
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

    pub fn claim_rewards(ctx: Context<ClaimRewards>, _market_id: u32) -> Result<()> {
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
    /// - On placing Order
    ///   - SELL order: Seller's YES/NO tokens locked in escrow immediately
    ///   - BUY order: Buyer's collateral locked in vault immediately
    /// - When matched:
    ///   - Buyer's claimable_yes incremented in their UserStats (can claim later from dashboard)
    ///   - Seller will withdraw collateral from vault separately
    pub fn place_order(
        ctx: Context<PlaceOrder>,
        side: OrderSide,
        token_type: TokenType,
        quantity: u64,
        max_iteration: u64,
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

        let (_mint_type, user_token_account, token_escrow) = match token_type {
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
        // For Buyer Lock collateral in Vault
        // For Seller Locking tokens in Escrow
        if side == OrderSide::Sell {
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

            let user_stats = &mut ctx.accounts.user_stats_account;

            match token_type {
                TokenType::Yes => {
                    user_stats.locked_yes = user_stats
                        .locked_yes
                        .checked_add(quantity)
                        .ok_or(PredictionMarketError::MathOverflow)?;
                }
                TokenType::No => {
                    user_stats.locked_no = user_stats
                        .locked_no
                        .checked_add(quantity)
                        .ok_or(PredictionMarketError::MathOverflow)?;
                }
            }
        } else {
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

            // Locking the collateral
            let user_stats = &mut ctx.accounts.user_stats_account;
            user_stats.locked_collateral = user_stats
                .locked_collateral
                .checked_add(amount)
                .ok_or(PredictionMarketError::MathOverflow)?;
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

        let mut idx = 0;
        let mut iteration = 0;
        let mut completed_orders: Vec<Order> = Vec::new();

        // Get the appropriate order vectors based on token type and side
        let (matching_orders, is_buy_order) = match (token_type, side) {
            (TokenType::Yes, OrderSide::Buy) => (&mut orderbook.yes_sell_orders, true),
            (TokenType::Yes, OrderSide::Sell) => (&mut orderbook.yes_buy_orders, false),
            (TokenType::No, OrderSide::Buy) => (&mut orderbook.no_sell_orders, true),
            (TokenType::No, OrderSide::Sell) => (&mut orderbook.no_buy_orders, false),
        };

        // Generalized matching logic for both YES and NO tokens
        while idx < matching_orders.len() && iteration <= max_iteration {
            let (book_price, book_qty, book_filled_qty) = {
                let book_order = &matching_orders[idx];
                (
                    book_order.price,
                    book_order.quantity,
                    book_order.filledquantity,
                )
            };

            // Price matching logic:
            // Buy order willing to pay UP TO price, so match if book price <= our price
            // Sell order willing to accept DOWN TO price, so match if book price >= our price
            let price_matches = if is_buy_order {
                order.price >= book_price // Buyer matches with lower or equal sell prices
            } else {
                order.price <= book_price // Seller matches with higher or equal buy prices
            };

            if price_matches {
                // Calculate remaining quantities
                let our_left_qty = order
                    .quantity
                    .checked_sub(order.filledquantity)
                    .ok_or(PredictionMarketError::MathOverflow)?;
                let book_left_qty = book_qty
                    .checked_sub(book_filled_qty)
                    .ok_or(PredictionMarketError::MathOverflow)?;
                let min_qty = our_left_qty.min(book_left_qty);

                if min_qty == 0 {
                    idx += 1;
                    continue;
                }

                // Update filled quantities
                matching_orders[idx].filledquantity = book_filled_qty
                    .checked_add(min_qty)
                    .ok_or(PredictionMarketError::MathOverflow)?;

                order.filledquantity = order
                    .filledquantity
                    .checked_add(min_qty)
                    .ok_or(PredictionMarketError::MathOverflow)?;

                let collateral_amount = min_qty
                    .checked_mul(book_price)
                    .ok_or(PredictionMarketError::MathOverflow)?;

                // Credit the appropriate user stats based on whether this is a buy or sell order
                if is_buy_order {
                    // When user is BUYER - credit YES/NO tokens
                    match token_type {
                        TokenType::Yes => {
                            ctx.accounts.user_stats_account.claimable_yes = ctx
                                .accounts
                                .user_stats_account
                                .claimable_yes
                                .checked_add(min_qty)
                                .ok_or(PredictionMarketError::MathOverflow)?;
                        }
                        TokenType::No => {
                            ctx.accounts.user_stats_account.claimable_no = ctx
                                .accounts
                                .user_stats_account
                                .claimable_no
                                .checked_add(min_qty)
                                .ok_or(PredictionMarketError::MathOverflow)?;
                        }
                    }

                    // Credit SELLER (from matching order) with collateral
                    let seller_pubkey = matching_orders[idx].user_key;
                    let seller_stats_pda = Pubkey::find_program_address(
                        &[
                            b"user_stats",
                            seller_pubkey.as_ref(),
                            market.market_id.to_le_bytes().as_ref(),
                        ],
                        ctx.program_id,
                    )
                    .0;

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
                        "Trade: Buyer +{} claimable {:?}, Seller +{} claimable collateral",
                        min_qty,
                        token_type,
                        collateral_amount
                    );
                } else {
                    // When user is SELLER - credit collateral
                    ctx.accounts.user_stats_account.claimable_collateral = ctx
                        .accounts
                        .user_stats_account
                        .claimable_collateral
                        .checked_add(collateral_amount)
                        .ok_or(PredictionMarketError::MathOverflow)?;

                    // Credit BUYER (from matching order) with YES/NO tokens
                    let buyer_pubkey = matching_orders[idx].user_key;
                    let buyer_stats_pda = Pubkey::find_program_address(
                        &[
                            b"user_stats",
                            buyer_pubkey.as_ref(),
                            market.market_id.to_le_bytes().as_ref(),
                        ],
                        ctx.program_id,
                    )
                    .0;

                    let mut buyer_credited = false;
                    for account_info in ctx.remaining_accounts.iter() {
                        if account_info.key == &buyer_stats_pda {
                            let mut data = account_info.try_borrow_mut_data()?;
                            let mut buyer_stats = UserStats::try_deserialize(&mut &data[..])?;

                            match token_type {
                                TokenType::Yes => {
                                    buyer_stats.claimable_yes = buyer_stats
                                        .claimable_yes
                                        .checked_add(min_qty)
                                        .ok_or(PredictionMarketError::MathOverflow)?;
                                }
                                TokenType::No => {
                                    buyer_stats.claimable_no = buyer_stats
                                        .claimable_no
                                        .checked_add(min_qty)
                                        .ok_or(PredictionMarketError::MathOverflow)?;
                                }
                            }

                            let mut writer = &mut data[..];
                            buyer_stats.try_serialize(&mut writer)?;

                            buyer_credited = true;
                            break;
                        }
                    }

                    require!(
                        buyer_credited,
                        PredictionMarketError::BuyerStatsAccountNotProvided
                    );

                    msg!(
                        "Trade: Seller +{} claimable collateral, Buyer +{} claimable {:?}",
                        collateral_amount,
                        min_qty,
                        token_type
                    );
                }

                // Remove completed orders
                if matching_orders[idx].filledquantity == matching_orders[idx].quantity {
                    let completed_order = matching_orders.remove(idx);
                    completed_orders.push(completed_order);
                    // Don't increment idx since we removed the element
                } else {
                    idx += 1;
                }

                iteration += 1;
            } else {
                // No more matching orders
                break;
            }
        }

        // Sorting Buy order in Decrement & Sell orders in Increment acc. to price
        if is_buy_order {
            matching_orders.sort_by(|a, b| a.price.cmp(&b.price));
        } else {
            matching_orders.sort_by(|a, b| b.price.cmp(&a.price));
        }

        // If order is not fully filled, add it to the appropriate order book
        if order.filledquantity < order.quantity {
            let order_vec = match (token_type, side) {
                (TokenType::Yes, OrderSide::Buy) => &mut orderbook.yes_buy_orders,
                (TokenType::Yes, OrderSide::Sell) => &mut orderbook.yes_sell_orders,
                (TokenType::No, OrderSide::Buy) => &mut orderbook.no_buy_orders,
                (TokenType::No, OrderSide::Sell) => &mut orderbook.no_sell_orders,
            };

            order_vec.push(order);

            // Sorting Buy order in Decrement & Sell orders in Increment acc. to price
            if side == OrderSide::Buy {
                order_vec.sort_by(|a, b| b.price.cmp(&a.price));
            } else {
                order_vec.sort_by(|a, b| a.price.cmp(&b.price));
            }
        }

        msg!(
            "Order processed: {} filled, {} remaining",
            order.filledquantity,
            order.quantity - order.filledquantity
        );

        Ok(())
    }
}

// Things remaining
// We will have to remove the orders whose filled quantity == quantity // Check more on this
// pushing things in the completed orders
// Then what we will do with Completed orders, We have to think about that
