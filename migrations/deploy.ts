import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PayChain } from "../target/types/pay_chain";
import { PublicKey, SystemProgram, Keypair } from "@solana/web3.js";
import * as dotenv from "dotenv";

dotenv.config();

module.exports = async function (provider) {
  // Configure client to use the provider.
  anchor.setProvider(provider);

  const program = anchor.workspace.PayChain as Program<PayChain>;
  
  // PDAs
  const [configPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("config")],
    program.programId
  );

  // Chains config
  // Default to Devnet if not specified
  const CHAIN_ID = process.env.CHAIN_ID || "solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1"; 
  const FEE_RECIPIENT = process.env.FEE_RECIPIENT ? new PublicKey(process.env.FEE_RECIPIENT) : provider.wallet.publicKey;
  const ROUTER = process.env.CCIP_ROUTER_ADDRESS ? new PublicKey(process.env.CCIP_ROUTER_ADDRESS) : Keypair.generate().publicKey; // Mock if missing

  try {
    const configAccount = await program.account.config.fetch(configPda);
    console.log("Program already initialized. Config:", configAccount);
  } catch (e) {
    console.log("Initializing Program with Chain ID:", CHAIN_ID);
    
    await program.methods
      .initialize(ROUTER, CHAIN_ID)
      .accounts({
        authority: provider.wallet.publicKey,
        feeRecipient: FEE_RECIPIENT,
        config: configPda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
      
    console.log("Initialization Complete!");
  }
};
