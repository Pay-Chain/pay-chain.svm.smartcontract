use anchor_lang::prelude::*;
use crate::state::*;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    /// CHECK: Fee recipient account
    pub fee_recipient: UncheckedAccount<'info>,
    
    #[account(
        init,
        payer = authority,
        space = 8 + Config::INIT_SPACE,
        seeds = [b"config"],
        bump
    )]
    pub config: Account<'info, Config>,
    
    pub system_program: Program<'info, System>,
}

pub fn initialize(ctx: Context<Initialize>, router: Pubkey, chain_id: String) -> Result<()> {
    let config = &mut ctx.accounts.config;
    config.authority = ctx.accounts.authority.key();
    config.fee_recipient = ctx.accounts.fee_recipient.key();
    config.router = router;
    config.chain_id = chain_id;
    config.fixed_base_fee = 500_000; // $0.50 in 6 decimals
    config.fee_rate_bps = 30; // 0.3%
    config.bump = ctx.bumps.config;
    Ok(())
}
