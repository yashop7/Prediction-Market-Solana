import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PredictionMarket } from "../target/types/prediction_market";
import { PublicKey, Keypair, SystemProgram, Connection } from "@solana/web3.js";

describe("prediction_market", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.predictionMarket as Program<PredictionMarket>;

  let authority = provider.wallet;
  let user : Keypair;


  // Mints and Accounts
  let collateralMint = PublicKey;
  let collateralVault = PublicKey;
  let outcomeAMint = PublicKey;
  let outcomeBMint = PublicKey;
  
  // User Account
  let userCollateralAccount = PublicKey;
  let userOutcomeAAccount : PublicKey;
  let userOutcomeBAccount : PublicKey;

  let marketId = 1;
  const initialCollateralAmount = 10000000;

  before(async () => {
    
  })

  it("Is initialized!", async () => {
    const provider = anchor
    console.log("Your transaction signature");
  });
});
