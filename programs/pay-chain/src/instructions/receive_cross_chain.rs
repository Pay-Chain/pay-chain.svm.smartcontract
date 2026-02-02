use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::state::*;
use crate::events::*;
use crate::errors::*;

// CCIP Seeds
pub const EXTERNAL_EXECUTION_CONFIG_SEED: &[u8] = b"external_execution_config";
pub const ALLOWED_OFFRAMP_SEED: &[u8] = b"allowed_offramp";

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, InitSpace, Debug)]
pub struct Any2SVMMessage {
    pub message_id: [u8; 32],
    pub source_chain_selector: u64,
    pub sender: [u8; 32],
    pub data: Vec<u8>,
    pub dest_token_amounts: Vec<SVMTokenAmount>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, InitSpace, Debug)]
pub struct SVMTokenAmount {
    pub token: Pubkey,
    pub amount: u64,
}

#[derive(Accounts)]
#[instruction(message: Any2SVMMessage)]
pub struct CcipReceive<'info> {
    // CCIP Offramp authority verification
    #[account(
        seeds = [EXTERNAL_EXECUTION_CONFIG_SEED, crate::ID.as_ref()],
        bump,
        seeds::program = offramp_program.key(),
    )]
    pub authority: Signer<'info>,

    /// CHECK: Offramp program info for verification
    pub offramp_program: UncheckedAccount<'info>,

    /// CHECK: Allowed offramp verification PDA
    #[account(
        seeds = [
            ALLOWED_OFFRAMP_SEED,
            message.source_chain_selector.to_le_bytes().as_ref(),
            offramp_program.key().as_ref()
        ],
        bump,
        seeds::program = config.router,
    )]
    pub allowed_offramp: UncheckedAccount<'info>,

    #[account(seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, Config>,

    #[account(
        mut,
        seeds = [b"vault", config.key().as_ref()],
        bump
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub receiver_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn ccip_receive(ctx: Context<CcipReceive>, message: Any2SVMMessage) -> Result<()> {
    // Verification: The CCIP logic is handled by the account constraints (authority and allowed_offramp)
    // Similar to EVM's onlyRouter modifier
    
    // Decode message data
    // Format: abi.encode(paymentId, amount, receiver)
    // paymentId (32 bytes), amount (32 bytes), receiver (32 bytes)
    let data = &message.data;
    if data.len() < 96 {
        return Err(PayChainError::InvalidMessageData.into());
    }

    let mut payment_id = [0u8; 32];
    payment_id.copy_from_slice(&data[0..32]);

    let mut amount_bytes = [0u8; 32];
    amount_bytes.copy_from_slice(&data[32..64]);
    let amount = u64::from_be_bytes(amount_bytes[24..32].try_into().unwrap());

    let mut receiver_bytes = [0u8; 32];
    receiver_bytes.copy_from_slice(&data[64..96]);
    // Unused in direct transfer but might be needed for validation
    // let receiver = Pubkey::new_from_array(receiver_bytes);

    // Payout to receiver (merchant)
    // Note: In a real implementation, we would transfer tokens from the vault
    let seeds = &[
        b"vault",
        ctx.accounts.config.to_account_info().key.as_ref(),
        &[ctx.accounts.config.bump],
    ];
    let signer = &[&seeds[..]];

    let cpi_accounts = Transfer {
        from: ctx.accounts.vault_token_account.to_account_info(),
        to: ctx.accounts.receiver_token_account.to_account_info(),
        authority: ctx.accounts.config.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    token::transfer(
        CpiContext::new_with_signer(cpi_program, cpi_accounts, signer),
        amount,
    )?;

    emit!(PaymentCompleted {
        payment_id,
        tx_hash: "".to_string(), // Transaction hash is available in context if needed
    });

    Ok(())
}
