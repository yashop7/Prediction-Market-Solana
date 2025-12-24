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
  let outcomeAMint: PublicKey;
  let outcomeBMint: PublicKey;
  let marketPda: PublicKey;

  // User Account
  let userCollateralAccount: PublicKey;
  let userOutcomeAAccount: PublicKey;
  let userOutcomeBAccount: PublicKey;

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
      [outcomeAMint] = PublicKey.findProgramAddressSync(
        [Buffer.from("outcome_a"), marketIdLE],
        program.programId
      );
      [outcomeBMint] = PublicKey.findProgramAddressSync(
        [Buffer.from("outcome_b"), marketIdLE],
        program.programId
      );

      // Now you can see all the accounts needed for initializeMarket!
      await program.methods
        .initializeMarket(marketId, settlementDeadline)
        .accounts({
          market: marketPda,
          authority: authority.publicKey,
          collateralMint: collateralMint,
          collateralVault,
          outcomeAMint,
          outcomeBMint,
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
        outcomeAMint,
        user.publicKey
      );
      userOutcomeAAccount = outcomeAAccountInfo.address;

      let outcomeBAccountInfo = await getOrCreateAssociatedTokenAccount(
        provider.connection,
        authority.payer,
        outcomeBMint,
        user.publicKey
      );
      userOutcomeBAccount = outcomeBAccountInfo.address;

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
          outcomeAMint,
          outcomeBMint,
          userOutcomeA: userOutcomeAAccount,
          userOutcomeB: userOutcomeBAccount,
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
        userOutcomeAAccount
      );
      let outcomeBAccount = await getAccount(
        provider.connection,
        userOutcomeBAccount
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
            outcomeAMint,
            outcomeBMint,
            userOutcomeA: userOutcomeAAccount,
            userOutcomeB: userOutcomeBAccount,
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
      let userOutcomeAAccountInfoBefore = await getAccount(
        provider.connection,
        userOutcomeAAccount
      );
      let userOutcomeBAccountInfoBefore = await getAccount(
        provider.connection,
        userOutcomeBAccount
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
          outcomeAMint,
          outcomeBMint,
          userOutcomeA: userOutcomeAAccount,
          userOutcomeB: userOutcomeBAccount,
          userCollateral: userCollateralAccount,
          collateralVault,
        })
        .signers([user])
        .rpc();

      console.log("merge Token is called");

      let mergeAmount = Math.min(
        Number(userOutcomeAAccountInfoBefore.amount),
        Number(userOutcomeBAccountInfoBefore.amount)
      );

      let userOutcomeAAccountInfoAfter = await getAccount(
        provider.connection,
        userOutcomeAAccount
      );
      let userOutcomeBAccountInfoAfter = await getAccount(
        provider.connection,
        userOutcomeBAccount
      );
      let userCollateralAccountInfoAfter = await getAccount(
        provider.connection,
        userCollateralAccount
      );

      assert.equal(
        Number(userOutcomeAAccountInfoBefore.amount) -
          Number(userOutcomeAAccountInfoAfter.amount),
        mergeAmount
      );
      assert.equal(
        Number(userOutcomeBAccountInfoBefore.amount) -
          Number(userOutcomeBAccountInfoAfter.amount),
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
          outcomeAMint,
          outcomeBMint,
          userOutcomeA: userOutcomeAAccount,
          userOutcomeB: userOutcomeBAccount,
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
    let winnerOutcomeAAccount: PublicKey;
    let winnerOutcomeBAccount: PublicKey;

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

      const winnerOutcomeAAccountInfo = await getOrCreateAssociatedTokenAccount(
        provider.connection,
        authority.payer,
        outcomeAMint,
        winningUser.publicKey
      );
      winnerOutcomeAAccount = winnerOutcomeAAccountInfo.address;
      const winnerOutcomeBAccountInfo = await getOrCreateAssociatedTokenAccount(
        provider.connection,
        authority.payer,
        outcomeBMint,
        winningUser.publicKey
      );
      winnerOutcomeBAccount = winnerOutcomeBAccountInfo.address;
      await program.methods
        .splitTokens(marketId, new BN(3000000))
        .accounts({
          market: marketPda,
          user: winningUser.publicKey,
          userCollateral: winnerCollateralAcount,
          collateralVault,
          outcomeAMint,
          outcomeBMint,
          userOutcomeA: winnerOutcomeAAccount,
          userOutcomeB: winnerOutcomeBAccount,
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
          outcomeAMint,
          outcomeBMint,
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
            outcomeAMint,
            outcomeBMint,
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
        winnerOutcomeAAccount
      );

      let rewardAmount = Number(winnerOutcomeABefore.amount);

      await program.methods
        .claimRewards(marketId)
        .accounts({
          market: marketPda,
          user: winningUser.publicKey,
          userCollateral: winnerCollateralAcount,
          collateralVault,
          outcomeAMint,
          outcomeBMint,
          userOutcomeA: winnerOutcomeAAccount,
          userOutcomeB: winnerOutcomeBAccount,
        })
        .signers([winningUser])
        .rpc();

      let winnerCollateralAcountAfter = await getAccount(
        provider.connection,
        winnerCollateralAcount
      );
      let winnerOutcomeAAfterClaiming = await getAccount(
        provider.connection,
        winnerOutcomeAAccount
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
            outcomeAMint,
            outcomeBMint,
            userOutcomeA: winnerOutcomeAAccount,
            userOutcomeB: winnerOutcomeBAccount,
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
            outcomeAMint,
            outcomeBMint,
            userOutcomeA: userOutcomeAAccount,
            userOutcomeB: userOutcomeBAccount,
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
          outcomeAMint,
          outcomeBMint,
          userOutcomeA: userOutcomeAAccount,
          userOutcomeB: userOutcomeBAccount,
        });
        assert.fail("Failing with this error MarketAlreadySettled");
      } catch (error) {
        expect(error.toString()).to.include("MarketAlreadySettled");
      }
    });
  });
});
