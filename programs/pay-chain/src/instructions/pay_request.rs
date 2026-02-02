use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::state::*;
use crate::events::*;
use crate::errors::*;

#[derive(Accounts)]
#[instruction(request_id: [u8; 32])]
pub struct PayRequest<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        seeds = [b"payment_request", request_id.as_ref()],
        bump = payment_request.bump
    )]
    pub payment_request: Account<'info, PaymentRequest>,

    #[account(mut)]
    pub payer_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub merchant_vault_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn pay_request(ctx: Context<PayRequest>, request_id: [u8; 32]) -> Result<()> {
    let request = &mut ctx.accounts.payment_request;
    require!(!request.is_paid, PayChainError::AlreadyPaid);
    require!(
        Clock::get()?.unix_timestamp <= request.expires_at,
        PayChainError::RequestExpired
    );

    // Transfer tokens from payer to merchant vault
    let cpi_accounts = Transfer {
        from: ctx.accounts.payer_token_account.to_account_info(),
        to: ctx.accounts.merchant_vault_token_account.to_account_info(),
        authority: ctx.accounts.payer.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    token::transfer(CpiContext::new(cpi_program, cpi_accounts), request.amount)?;

    request.is_paid = true;
    request.payer = Some(ctx.accounts.payer.key());

    emit!(RequestPaymentReceived {
        request_id,
        payer: ctx.accounts.payer.key(),
    });

    Ok(())
}
