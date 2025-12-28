# Prediction Market on Solana

This is a decentralized prediction market built on Solana using the Anchor framework. The idea is simple: people can bet on binary outcomes (like "will it rain tomorrow?" or "will team A win?") by buying outcome tokens, and when the event resolves, winners can claim their rewards.

## What This Does

The smart contract lets you create markets where there are two possible outcomes (A or B). Users deposit collateral (like USDC) and get an equal amount of both outcome tokens. If you think outcome A will win, you'd sell your B tokens and hold A tokens. When the market settles, whoever holds the winning tokens can redeem them for the collateral at a 1:1 ratio.

Think of it like this: you deposit 100 USDC, you get 100 A tokens and 100 B tokens. If you believe in outcome A, you sell your B tokens to someone else. If A wins, you redeem your 100 A tokens for 100 USDC. The person who bought your B tokens loses their bet.

## Core Features

**Market Creation**: Anyone can initialize a new prediction market with a unique ID, settlement deadline, and collateral token. The contract automatically creates the outcome token mints and a vault for holding collateral.

**Split Tokens**: Users deposit collateral and receive an equal amount of both outcome tokens. This is the entry point for participating in any market.

**Merge Tokens**: If you want to exit your position before settlement, you can burn equal amounts of both outcome tokens to get your collateral back. Basically it's a refund mechanism.

**Settlement**: The market authority sets the winning outcome after the deadline. This freezes the market and prevents any new minting of outcome tokens.

**Claim Rewards**: After settlement, users with winning tokens can claim their share of the collateral vault. The redemption ratio depends on the winning outcome.

## How It Works Technically

The contract uses Program Derived Addresses (PDAs) for all the accounts, which means everything is deterministic and secure. The market PDA acts as the mint authority for both outcome tokens, and it also controls the collateral vault.

When you split tokens, the contract transfers your collateral to the vault and mints you outcome tokens. When you merge, it burns your outcome tokens and returns the collateral. After settlement, only the winning tokens are redeemable for collateral.

The contract also handles edge cases like draws (where the outcome is "Neither"), in which case both token types might get partial payouts, though the exact mechanism for that would need to be configured based on your use case.

## Project Structure

The actual Solana program is in the `contract` folder. The main logic lives in these files:

- `lib.rs` has all the instruction handlers (initialize, split, merge, settle, claim)
- `state.rs` defines the Market account structure and the WinningOutcome enum
- `instructions.rs` contains all the account validation structs for each instruction
- `error.rs` has custom error types for better debugging

## Building and Testing

You'll need Rust, Solana CLI, and Anchor installed. The project uses Anchor 0.30.0 or whatever version is in the Anchor.toml.

To build:
```
cd contract
anchor build
```

To test:
```
anchor test
```

The tests are written in TypeScript and they cover the full lifecycle of a market: initialization, users splitting tokens, trading between users, settlement, and claiming rewards.

## Deployment

You can deploy this to devnet, testnet, or mainnet. Just make sure to update the program ID after your first build and before deploying for real. The current program ID is in `lib.rs` and was generated during initial development.

```
anchor deploy
```

Make sure you have enough SOL for deployment fees and rent exemption.

## Security Considerations

This is still a work in progress and hasn't been audited. Some things to keep in mind:

- The market authority has full control over settlement, so it's centralized right now. You'd want to add governance or an oracle system for production.
- There's no fee mechanism built in yet, so market creators don't get compensated for creating markets.
- The settlement deadline is checked when users try to interact with the market, but the authority can still set the winning outcome before the deadline expires, which might not be ideal.
- Math operations use checked arithmetic to prevent overflows, which is good.

## Future Improvements

Some ideas for extending this:

- Add an oracle integration for automatic settlement
- Implement a fee system (maybe a small percentage goes to the market creator)
- Support for more than two outcomes
- Automated market maker for trading outcome tokens without needing external liquidity
- Better metadata support (right now the Market struct has a comment about adding a metadata URL, but it's not implemented)

## License

Do whatever you want with this code. No warranties, use at your own risk.
