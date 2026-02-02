use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::state::*;
use crate::events::*;
use crate::errors::*;

#[derive(Accounts)]
pub struct ProcessRefund<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    #[account(seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, Config>,

    #[account(
        mut,
        seeds = [b"payment", payment.payment_id.as_ref()],
        bump = payment.bump
    )]
    pub payment: Account<'info, Payment>,

    #[account(
        mut,
        seeds = [b"vault", config.key().as_ref()],
        bump
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub sender_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn process_refund(ctx: Context<ProcessRefund>) -> Result<()> {
    let payment = &mut ctx.accounts.payment;
    
    require!(
        payment.status == PaymentStatus::Failed,
        PayChainError::PaymentNotFailed
    );

    payment.status = PaymentStatus::Refunded;

    // Transfer funds back to sender
    let seeds = &[
        b"vault",
        ctx.accounts.config.to_account_info().key.as_ref(),
        &[ctx.accounts.config.bump],
    ];
    let signer = &[&seeds[..]];

    let cpi_accounts = Transfer {
        from: ctx.accounts.vault_token_account.to_account_info(),
        to: ctx.accounts.sender_token_account.to_account_info(),
        authority: ctx.accounts.config.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    token::transfer(
        CpiContext::new_with_signer(cpi_program, cpi_accounts, signer),
        payment.amount,
    )?;

    emit!(PaymentRefunded {
        payment_id: payment.payment_id,
        refund_amount: payment.amount,
    });

    Ok(())
}
