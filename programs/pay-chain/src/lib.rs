use anchor_lang::prelude::*;

pub mod errors;
pub mod events;
pub mod instructions;
pub mod state;

use instructions::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod pay_chain {
    use super::*;

    /// Initialize the PayChain program
    pub fn initialize(ctx: Context<Initialize>, router: Pubkey, chain_id: String) -> Result<()> {
        instructions::initialize(ctx, router, chain_id)
    }

    /// Create a new cross-chain payment (Solana -> EVM)
    pub fn create_payment(
        ctx: Context<CreatePayment>,
        payment_id: [u8; 32],
        dest_chain_id: String,
        dest_token: [u8; 32],
        amount: u64,
        receiver: [u8; 32],
    ) -> Result<()> {
        instructions::create_payment(
            ctx,
            payment_id,
            dest_chain_id,
            dest_token,
            amount,
            receiver,
        )
    }

    /// CCIP Receiver Instruction (EVM -> Solana)
    pub fn ccip_receive(ctx: Context<CcipReceive>, message: Any2SVMMessage) -> Result<()> {
        instructions::ccip_receive(ctx, message)
    }

    /// Create a payment request (Merchant creates a request for a customer)
    pub fn create_payment_request(
        ctx: Context<CreatePaymentRequest>,
        request_id: [u8; 32],
        token: Pubkey,
        amount: u64,
        description: String,
    ) -> Result<()> {
        instructions::create_payment_request(ctx, request_id, token, amount, description)
    }

    /// Pay a payment request (Customer pays the merchant request on-chain)
    pub fn pay_request(ctx: Context<PayRequest>, request_id: [u8; 32]) -> Result<()> {
        instructions::pay_request(ctx, request_id)
    }

    /// Process refund for failed payment
    pub fn process_refund(ctx: Context<ProcessRefund>) -> Result<()> {
        instructions::process_refund(ctx)
    }

    /// Swap tokens (Generic CPI wrapper for Jupiter/DEX)
    pub fn swap_tokens(ctx: Context<SwapTokens>, data: Vec<u8>) -> Result<()> {
        instructions::swap_tokens(ctx, data)
    }
}
