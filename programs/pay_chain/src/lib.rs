use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod pay_chain {
    use super::*;

    /// Initialize the PayChain program
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.authority = ctx.accounts.authority.key();
        config.fee_recipient = ctx.accounts.fee_recipient.key();
        config.fixed_base_fee = 500_000; // $0.50 in 6 decimals
        config.fee_rate_bps = 30; // 0.3%
        config.bump = ctx.bumps.config;
        Ok(())
    }

    /// Create a new cross-chain payment
    pub fn create_payment(
        ctx: Context<CreatePayment>,
        payment_id: [u8; 32],
        dest_chain_id: String,
        amount: u64,
        receiver: Pubkey,
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
        payment.receiver = receiver;
        payment.source_chain_id = "solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1".to_string();
        payment.dest_chain_id = dest_chain_id;
        payment.amount = amount;
        payment.fee = fee;
        payment.status = PaymentStatus::Pending;
        payment.created_at = Clock::get()?.unix_timestamp;
        payment.bump = ctx.bumps.payment;

        emit!(PaymentCreated {
            payment_id,
            sender: ctx.accounts.sender.key(),
            receiver,
            amount,
            fee,
        });

        Ok(())
    }

    /// Process refund for failed payment (amount only, fee not refunded)
    pub fn process_refund(ctx: Context<ProcessRefund>) -> Result<()> {
        let payment = &mut ctx.accounts.payment;
        
        require!(
            payment.status == PaymentStatus::Failed,
            PayChainError::PaymentNotFailed
        );

        payment.status = PaymentStatus::Refunded;

        emit!(PaymentRefunded {
            payment_id: payment.payment_id,
            refund_amount: payment.amount, // Only amount, not fee
        });

        Ok(())
    }
}

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
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ProcessRefund<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    #[account(
        mut,
        seeds = [b"payment", payment.payment_id.as_ref()],
        bump = payment.bump
    )]
    pub payment: Account<'info, Payment>,
}

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub authority: Pubkey,
    pub fee_recipient: Pubkey,
    pub fixed_base_fee: u64,
    pub fee_rate_bps: u16,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Payment {
    pub payment_id: [u8; 32],
    pub sender: Pubkey,
    pub receiver: Pubkey,
    #[max_len(64)]
    pub source_chain_id: String,
    #[max_len(64)]
    pub dest_chain_id: String,
    pub amount: u64,
    pub fee: u64,
    pub status: PaymentStatus,
    pub created_at: i64,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, InitSpace)]
pub enum PaymentStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Refunded,
}

#[event]
pub struct PaymentCreated {
    pub payment_id: [u8; 32],
    pub sender: Pubkey,
    pub receiver: Pubkey,
    pub amount: u64,
    pub fee: u64,
}

#[event]
pub struct PaymentRefunded {
    pub payment_id: [u8; 32],
    pub refund_amount: u64,
}

#[error_code]
pub enum PayChainError {
    #[msg("Payment is not in failed status")]
    PaymentNotFailed,
    #[msg("Unauthorized")]
    Unauthorized,
}
