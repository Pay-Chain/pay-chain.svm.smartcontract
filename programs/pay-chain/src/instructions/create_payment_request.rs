use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use crate::state::*;
use crate::events::*;

#[derive(Accounts)]
#[instruction(request_id: [u8; 32])]
pub struct CreatePaymentRequest<'info> {
    #[account(mut)]
    pub merchant: Signer<'info>,

    /// CHECK: Merchant vault token account
    pub merchant_vault: UncheckedAccount<'info>,

    #[account(
        init,
        payer = merchant,
        space = 8 + PaymentRequest::INIT_SPACE,
        seeds = [b"payment_request", request_id.as_ref()],
        bump
    )]
    pub payment_request: Account<'info, PaymentRequest>,

    pub system_program: Program<'info, System>,
}

pub fn create_payment_request(
    ctx: Context<CreatePaymentRequest>,
    request_id: [u8; 32],
    token: Pubkey,
    amount: u64,
    description: String,
) -> Result<()> {
    let request = &mut ctx.accounts.payment_request;
    request.merchant = ctx.accounts.merchant.key();
    request.receiver = ctx.accounts.merchant_vault.key();
    request.token = token;
    request.amount = amount;
    request.description = description;
    request.is_paid = false;
    request.expires_at = Clock::get()?.unix_timestamp + 900; // 15 minutes
    request.bump = ctx.bumps.payment_request;

    emit!(PaymentRequestCreated {
        request_id,
        merchant: ctx.accounts.merchant.key(),
        amount,
        description: request.description.clone(),
    });

    Ok(())
}
