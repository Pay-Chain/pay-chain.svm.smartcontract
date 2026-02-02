use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use crate::state::*;

#[derive(Accounts)]
pub struct SwapTokens<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: Jupiter/Dex Program ID
    pub swap_program: UncheckedAccount<'info>,

    #[account(mut)]
    pub source_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub dest_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    
    // Remaining accounts required by the specific DEX (pools, vaults, etc.)
}

pub fn swap_tokens(ctx: Context<SwapTokens>, data: Vec<u8>) -> Result<()> {
    // Generic CPI wrapper to invoke generic Swap programs (Jupiter, Orca, etc.)
    // This allows bundling Swap + Pay in a verified way if needed, 
    // or simply providing an on-chain interface for the protocol.

    let program = ctx.accounts.swap_program.to_account_info();
    let accounts: Vec<AccountMeta> = ctx
        .remaining_accounts
        .iter()
        .map(|acc| AccountMeta {
            pubkey: *acc.key,
            is_signer: acc.is_signer,
            is_writable: acc.is_writable,
        })
        .collect();
    
    let account_infos: Vec<AccountInfo> = ctx
        .remaining_accounts
        .iter()
        .map(|acc| acc.to_account_info())
        .collect();

    // Create the instruction
    let instruction = anchor_lang::solana_program::instruction::Instruction {
        program_id: *program.key,
        accounts,
        data,
    };

    // Invoke the swap instruction
    // Note: If the swap requires the program's authority (PDAs), we would use invoke_signed.
    // Here we assume the user (payer) is the owner of the source funds or has delegated approval.
    anchor_lang::solana_program::program::invoke(&instruction, &account_infos)?;

    Ok(())
}
