import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PredictionMarket } from "../target/types/prediction_market";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  LAMPORTS_PER_SOL,
  Connection,
} from "@solana/web3.js";
import {
  createMint,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  TOKEN_PROGRAM_ID,
  getAccount,
} from "@solana/spl-token";
import { BN } from "bn.js";
import { assert, expect } from "chai";

describe("prediction_market", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace
    .predictionMarket as Program<PredictionMarket>;

  let authority = provider.wallet;
  let user: Keypair;

  // Mints and Accounts
  let collateralMint: PublicKey;
  let collateralVault: PublicKey;
  let outcomeYesMint: PublicKey;
  let outcomeNoMint: PublicKey;
  let marketPda: PublicKey;
  let orderbook: PublicKey;

  // User Account
  let userCollateralAccount: PublicKey;
  let userOutcomeYesAccount: PublicKey;
  let userOutcomeNoAccount: PublicKey;

  let marketId = 1;
  const initialCollateralAmount = 10000000;

  before(async () => {
    user = Keypair.generate();

    const airdropSignature = await provider.connection.requestAirdrop(
      user.publicKey,
      anchor.web3.LAMPORTS_PER_SOL * 2
    );

    await provider.connection.confirmTransaction(airdropSignature);

    collateralMint = await createMint(
      provider.connection,
      authority.payer,
      authority?.publicKey,
      null,
      6
    );

    console.log("Collateral Mint:", collateralMint.toBase58());
  });

  describe("Initialize Market", () => {
    it("Intialising the Prediction Market Succesfully", async () => {
      const settlementDeadline = new anchor.BN(
        Math.floor(Date.now() / 1000) + 86400
      );

      const marketID = new BN(1);
      const marketIdLE = marketID.toArrayLike(Buffer, "le", 4); // Converting it into 4-byte little-endian Buffer
      [marketPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("market"), marketIdLE],
        program.programId
      );
      [collateralVault] = PublicKey.findProgramAddressSync(
        [Buffer.from("vault"), marketIdLE],
        program.programId
      );
      [outcomeYesMint] = PublicKey.findProgramAddressSync(
        [Buffer.from("outcome_a"), marketIdLE],
        program.programId
      );
      [outcomeNoMint] = PublicKey.findProgramAddressSync(
        [Buffer.from("outcome_b"), marketIdLE],
        program.programId
      );
      [orderbook] = PublicKey.findProgramAddressSync(
        [Buffer.from("orderbook"), marketIdLE],
        program.programId
      )

      // Now you can see all the accounts needed for initializeMarket!
      await program.methods
        .initializeMarket(marketId, settlementDeadline)
        .accounts({
          market: marketPda,
          authority: authority.publicKey,
          collateralMint: collateralMint,
          collateralVault,
          outcomeYesMint,
          outcomeNoMint,
          orderbook,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        })
        .rpc();

      console.log("Market initialized successfully!");
      console.log("Market PDA:", marketPda.toBase58());
    });
  });

  describe("Split Tokens", () => {
    before(async () => {
      // What we want to do is that fund the User Mint Account
      let userCollateralAccountInfo = await getOrCreateAssociatedTokenAccount(
        provider.connection,
        authority.payer,
        collateralMint,
        user.publicKey
      );

      userCollateralAccount = userCollateralAccountInfo.address;
      // Now we will mint tokens in the user Account

      await mintTo(
        provider.connection,
        authority.payer,
        collateralMint,
        userCollateralAccount,
        authority.publicKey,
        initialCollateralAmount
      );
      console.log("Tokens Minted to the User wallet");

      let outcomeAAccountInfo = await getOrCreateAssociatedTokenAccount(
        provider.connection,
        authority.payer,
        outcomeYesMint,
        user.publicKey
      );
      userOutcomeYesAccount = outcomeAAccountInfo.address;

      let outcomeBAccountInfo = await getOrCreateAssociatedTokenAccount(
        provider.connection,
        authority.payer,
        outcomeNoMint,
        user.publicKey
      );
      userOutcomeNoAccount = outcomeBAccountInfo.address;

      console.log(
        "Now we have funded the User Collateral,Outcome Accounts & Also funded his Collateral Account"
      );
    });

    it("Split Collateral Account & fund both the Outcome Account of User", async () => {
      const splitAmount = 1000000;

      let userCollateralAccountBefore = await getAccount(
        provider.connection,
        userCollateralAccount
      );

      await program.methods
        .splitTokens(marketId, new anchor.BN(splitAmount))
        .accounts({
          market: marketPda,
          user: user.publicKey,
          userCollateral: userCollateralAccount,
          collateralVault,
          outcomeYesMint,
          outcomeNoMint,
          userOutcomeYes: userOutcomeYesAccount,
          userOutcomeNo: userOutcomeNoAccount,
        })
        .signers([user])
        .rpc();

      let userCollateralAccountAfter = await getAccount(
        provider.connection,
        userCollateralAccount
      );

      //Ok Now we have to verify does the tokens really Split

      //Getting the Outcome Account
      let outcomeAAccount = await getAccount(
        provider.connection,
        userOutcomeYesAccount
      );
      let outcomeBAccount = await getAccount(
        provider.connection,
        userOutcomeNoAccount
      );
      let vault = await getAccount(provider.connection, collateralVault);
      assert.equal(Number(outcomeAAccount.amount), splitAmount);
      assert.equal(Number(outcomeBAccount.amount), splitAmount);
      assert.equal(Number(vault.amount), splitAmount);
      //checking the user Balance
      assert.equal(
        Number(userCollateralAccountBefore.amount) -
          Number(userCollateralAccountAfter.amount),
        splitAmount
      );

      //Now the Market is initialised & we will verify the state of market, like how much is locked in the market right now
      const market = await program.account.market.fetch(marketPda);
      assert.equal(Number(market.totalCollateralLocked), splitAmount);
    });

    it("What If we give zero amount, then we will observe the State", async () => {
      try {
        const splitAmount = 0;
        await program.methods
          .splitTokens(marketId, new anchor.BN(splitAmount))
          .accounts({
            market: marketPda,
            user: user.publicKey,
            userCollateral: userCollateralAccount,
            collateralVault,
            outcomeYesMint,
            outcomeNoMint,
            userOutcomeYes: userOutcomeYesAccount,
            userOutcomeNo: userOutcomeNoAccount,
          })
          .signers([user])
          .rpc();

        // This call will obviously Fail
        assert.fail("Amount should be more than Zero");
      } catch (err) {
        expect(err.toString()).to.include("InvalidAmount");
      }
    });
  });

  describe("Merge Tokens", () => {
    // Now we will merge token
    it("Merges outcome tokens back to collateral", async () => {
      let userOutcomeYesAccountInfoBefore = await getAccount(
        provider.connection,
        userOutcomeYesAccount
      );
      let userOutcomeNoAccountInfoBefore = await getAccount(
        provider.connection,
        userOutcomeNoAccount
      );
      let userCollateralAccountInfoBefore = await getAccount(
        provider.connection,
        userCollateralAccount
      );

      await program.methods
        .mergeTokens(marketId)
        .accounts({
          market: marketPda,
          user: user.publicKey,
          outcomeYesMint,
          outcomeNoMint,
          userOutcomeYes: userOutcomeYesAccount,
          userOutcomeNo: userOutcomeNoAccount,
          userCollateral: userCollateralAccount,
          collateralVault,
        })
        .signers([user])
        .rpc();

      console.log("merge Token is called");

      let mergeAmount = Math.min(
        Number(userOutcomeYesAccountInfoBefore.amount),
        Number(userOutcomeNoAccountInfoBefore.amount)
      );

      let userOutcomeYesAccountInfoAfter = await getAccount(
        provider.connection,
        userOutcomeYesAccount
      );
      let userOutcomeNoAccountInfoAfter = await getAccount(
        provider.connection,
        userOutcomeNoAccount
      );
      let userCollateralAccountInfoAfter = await getAccount(
        provider.connection,
        userCollateralAccount
      );

      assert.equal(
        Number(userOutcomeYesAccountInfoBefore.amount) -
          Number(userOutcomeYesAccountInfoAfter.amount),
        mergeAmount
      );
      assert.equal(
        Number(userOutcomeNoAccountInfoBefore.amount) -
          Number(userOutcomeNoAccountInfoAfter.amount),
        mergeAmount
      );
      assert.equal(
        Number(userCollateralAccountInfoAfter.amount) -
          Number(userCollateralAccountInfoBefore.amount),
        mergeAmount
      );

      // Now we will check the market State
      const marketAccount = await program.account.market.fetch(marketPda);
      assert.equal(Number(marketAccount.totalCollateralLocked), 0);
      console.log("Tokens merged successfully");
    });

    it("Expected the Merge to Fail, No outcome token is present in User Account", async () => {
      // No Outcome tokens are left with User
      try {
        program.methods.mergeTokens(marketId).accounts({
          market: marketPda,
          user: user.publicKey,
          outcomeYesMint,
          outcomeNoMint,
          userOutcomeYes: userOutcomeYesAccount,
          userOutcomeNo: userOutcomeNoAccount,
          userCollateral: userCollateralAccount,
          collateralVault,
        });

        assert.fail("InvalidAmount");
      } catch (err) {
        expect(err.toString()).to.include("InvalidAmount");
      }
    });
  });

  describe("Claiming Rewards & Set Winning Side", () => {
    //Defining accounts of user & then funding them
    let winningUser: Keypair;
    let winnerCollateralAcount: PublicKey;
    let winnerOutcomeYesAccount: PublicKey;
    let winnerOutcomeNoAccount: PublicKey;

    before(async () => {
      winningUser = Keypair.generate();
      let requestAirdropSig = await provider.connection.requestAirdrop(
        winningUser.publicKey,
        anchor.web3.LAMPORTS_PER_SOL * 5
      );
      await provider.connection.confirmTransaction(requestAirdropSig);
      // Now we will fund the User Collateral Account
      const winnerCollateralAcountInfo =
        await getOrCreateAssociatedTokenAccount(
          provider.connection,
          authority.payer,
          collateralMint,
          winningUser.publicKey
        );
      winnerCollateralAcount = winnerCollateralAcountInfo.address;

      await mintTo(
        provider.connection,
        authority.payer,
        collateralMint,
        winnerCollateralAcount,
        authority.publicKey,
        5000000
      );

      const winnerOutcomeYesAccountInfo = await getOrCreateAssociatedTokenAccount(
        provider.connection,
        authority.payer,
        outcomeYesMint,
        winningUser.publicKey
      );
      winnerOutcomeYesAccount = winnerOutcomeYesAccountInfo.address;
      const winnerOutcomeNoAccountInfo = await getOrCreateAssociatedTokenAccount(
        provider.connection,
        authority.payer,
        outcomeNoMint,
        winningUser.publicKey
      );
      winnerOutcomeNoAccount = winnerOutcomeNoAccountInfo.address;
      await program.methods
        .splitTokens(marketId, new BN(3000000))
        .accounts({
          market: marketPda,
          user: winningUser.publicKey,
          userCollateral: winnerCollateralAcount,
          collateralVault,
          outcomeYesMint,
          outcomeNoMint,
          userOutcomeYes: winnerOutcomeYesAccount,
          userOutcomeNo: winnerOutcomeNoAccount,
        })
        .signers([winningUser])
        .rpc();
    });

    it("Setting the Winning Side to A", async () => {
      await program.methods
        .setWinningSide(marketId, { outcomeA: {} })
        .accounts({
          authority: authority.publicKey,
          market: marketPda,
          outcomeYesMint,
          outcomeNoMint,
        })
        .signers([authority.payer])
        .rpc();

      // Checking the Market State
      let marketInfo = await program.account.market.fetch(marketPda);
      assert.isTrue(marketInfo.isSettled);
      assert.deepEqual(marketInfo.winningOutcome, { outcomeA: {} });
    });
    it("Again setting the winning Side, Expecting the error this time", async () => {
      try {
        program.methods
          .setWinningSide(marketId, { outcomeB: {} })
          .accounts({
            authority: authority.publicKey,
            market: marketPda,
            outcomeYesMint,
            outcomeNoMint,
          })
          .signers([authority.payer])
          .rpc();
        assert.fail("MarketAlreadySettled");
      } catch (err) {
        expect(err.toString()).to.include("MarketAlreadySettled");
      }
    });

    it("Claiming Rewads for the other side", async () => {
      let winnerCollateralAcountBefore = await getAccount(
        provider.connection,
        winnerCollateralAcount
      );
      let winnerOutcomeABefore = await getAccount(
        provider.connection,
        winnerOutcomeYesAccount
      );

      let rewardAmount = Number(winnerOutcomeABefore.amount);

      await program.methods
        .claimRewards(marketId)
        .accounts({
          market: marketPda,
          user: winningUser.publicKey,
          userCollateral: winnerCollateralAcount,
          collateralVault,
          outcomeYesMint,
          outcomeNoMint,
          userOutcomeYes: winnerOutcomeYesAccount,
          userOutcomeNo: winnerOutcomeNoAccount,
        })
        .signers([winningUser])
        .rpc();

      let winnerCollateralAcountAfter = await getAccount(
        provider.connection,
        winnerCollateralAcount
      );
      let winnerOutcomeAAfterClaiming = await getAccount(
        provider.connection,
        winnerOutcomeYesAccount
      );

      //checking the winner collateral Account
      assert.equal(
        Number(winnerCollateralAcountAfter.amount) -
          Number(winnerCollateralAcountBefore.amount),
        rewardAmount
      );
      // Let's check also the Outcome Account to be 0
      assert.equal(Number(winnerOutcomeAAfterClaiming.amount), 0);

      console.log("Rewards are claimed by the User");
    });

    it("It should Fail, Claiming rewards Twice", async () => {
      try {
        program.methods
          .claimRewards(marketId)
          .accounts({
            market: marketPda,
            user: winningUser.publicKey,
            userCollateral: winnerCollateralAcount,
            collateralVault,
            outcomeYesMint,
            outcomeNoMint,
            userOutcomeYes: winnerOutcomeYesAccount,
            userOutcomeNo: winnerOutcomeNoAccount,
          })
          .signers([winningUser])
          .rpc();

        assert.fail("RewardAlreadyClaimed");
      } catch (err) {
        console.log("RewardAlreadyClaimed");
        expect(err.toString()).to.include("RewardAlreadyClaimed");
      }
    });
  });

  describe("Discusing Edge Cases", async () => {
    it("Let's see the tokens will still split after the market is Settled", async () => {
      try {
        await program.methods
          .splitTokens(marketId, new BN(10000))
          .accounts({
            market: marketPda,
            user: user.publicKey,
            userCollateral: userCollateralAccount,
            collateralVault,
            outcomeYesMint,
            outcomeNoMint,
            userOutcomeYes: userOutcomeYesAccount,
            userOutcomeNo: userOutcomeNoAccount,
          })
          .signers([user])
          .rpc();

        assert.fail("Failing with this error MarketAlreadySettled");
      } catch (err) {
        console.log("err.toString(): ", err.toString());
        expect(err.toString()).to.include("MarketAlreadySettled");
      }
    });

    it("Trying to merge Tokens after the market is settled", async () => {
      try {
        await program.methods.mergeTokens(marketId).accounts({
          market: marketPda,
          user: user.publicKey,
          userCollateral: userCollateralAccount,
          collateralVault,
          outcomeYesMint,
          outcomeNoMint,
          userOutcomeYes: userOutcomeYesAccount,
          userOutcomeNo: userOutcomeNoAccount,
        });
        assert.fail("Failing with this error MarketAlreadySettled");
      } catch (error) {
        expect(error.toString()).to.include("MarketAlreadySettled");
      }
    });
  });
});
