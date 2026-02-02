use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::state::*;
use crate::events::*;

#[derive(Accounts)]
#[instruction(payment_id: [u8; 32])]
pub struct CreatePayment<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,
    
    #[account(seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, Config>,
    
    #[account(
        init,
        payer = sender,
        space = 8 + Payment::INIT_SPACE,
        seeds = [b"payment", payment_id.as_ref()],
        bump
    )]
    pub payment: Account<'info, Payment>,

    #[account(mut)]
    pub sender_token_account: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = sender,
        seeds = [b"vault", config.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = vault_token_account,
    )]
    pub vault_token_account: Account<'info, TokenAccount>,
    
    pub mint: Account<'info, token::Mint>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn create_payment(
    ctx: Context<CreatePayment>,
    payment_id: [u8; 32],
    dest_chain_id: String,
    dest_token: [u8; 32],
    amount: u64,
    receiver: [u8; 32], // EVM address or Solana pubkey as bytes
) -> Result<()> {
    let payment = &mut ctx.accounts.payment;
    let config = &ctx.accounts.config;

    // Calculate fee
    let percentage_fee = (amount as u128 * config.fee_rate_bps as u128 / 10000) as u64;
    let fee = if percentage_fee > config.fixed_base_fee {
        percentage_fee
    } else {
        config.fixed_base_fee
    };

    payment.payment_id = payment_id;
    payment.sender = ctx.accounts.sender.key();
    // For SVM->EVM, receiver is the recipient's address on the dest chain
    payment.receiver_bytes = receiver;
    payment.source_chain_id = config.chain_id.clone();
    payment.dest_chain_id = dest_chain_id;
    payment.amount = amount;
    payment.fee = fee;
    payment.status = PaymentStatus::Pending;
    payment.created_at = Clock::get()?.unix_timestamp;
    payment.bump = ctx.bumps.payment;

    // Transfer tokens from sender to vault (PDA)
    // NOTE: In the struct validation, we added `mint` account.
    let cpi_accounts = Transfer {
        from: ctx.accounts.sender_token_account.to_account_info(),
        to: ctx.accounts.vault_token_account.to_account_info(),
        authority: ctx.accounts.sender.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    token::transfer(CpiContext::new(cpi_program, cpi_accounts), amount + fee)?;

    emit!(PaymentCreated {
        payment_id,
        sender: ctx.accounts.sender.key(),
        amount,
        fee,
    });

    Ok(())
}
