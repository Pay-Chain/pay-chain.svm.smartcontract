use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

// CCIP Seeds
pub const EXTERNAL_EXECUTION_CONFIG_SEED: &[u8] = b"external_execution_config";
pub const ALLOWED_OFFRAMP_SEED: &[u8] = b"allowed_offramp";

#[program]
pub mod pay_chain {
    use super::*;

    /// Initialize the PayChain program
    pub fn initialize(ctx: Context<Initialize>, router: Pubkey) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.authority = ctx.accounts.authority.key();
        config.fee_recipient = ctx.accounts.fee_recipient.key();
        config.router = router;
        config.fixed_base_fee = 500_000; // $0.50 in 6 decimals
        config.fee_rate_bps = 30; // 0.3%
        config.bump = ctx.bumps.config;
        Ok(())
    }

    /// Create a new cross-chain payment (Solana -> EVM)
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
        payment.source_chain_id = "solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1".to_string(); // Mainnet example
        payment.dest_chain_id = dest_chain_id;
        payment.amount = amount;
        payment.fee = fee;
        payment.status = PaymentStatus::Pending;
        payment.created_at = Clock::get()?.unix_timestamp;
        payment.bump = ctx.bumps.payment;

        // Transfer tokens from sender to vault (PDA)
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

    /// CCIP Receiver Instruction (EVM -> Solana)
    /// This is called by the CCIP offramp to deliver messages from EVM
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
        let receiver = Pubkey::new_from_array(receiver_bytes);

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

    /// Create a payment request (Merchant creates a request for a customer)
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

    /// Pay a payment request (Customer pays the merchant request on-chain)
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

    /// Process refund for failed payment
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

    #[account(mut)]
    pub sender_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"vault", config.key().as_ref()],
        bump
    )]
    pub vault_token_account: Account<'info, TokenAccount>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
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

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub authority: Pubkey,
    pub fee_recipient: Pubkey,
    pub router: Pubkey,
    pub fixed_base_fee: u64,
    pub fee_rate_bps: u16,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Payment {
    pub payment_id: [u8; 32],
    pub sender: Pubkey,
    pub receiver_bytes: [u8; 32],
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

#[account]
#[derive(InitSpace)]
pub struct PaymentRequest {
    pub merchant: Pubkey,
    pub receiver: Pubkey,
    pub token: Pubkey,
    pub amount: u64,
    #[max_len(128)]
    pub description: String,
    pub is_paid: bool,
    pub payer: Option<Pubkey>,
    pub expires_at: i64,
    pub bump: u8,
}

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

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, InitSpace, Debug)]
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
    pub amount: u64,
    pub fee: u64,
}

#[event]
pub struct PaymentCompleted {
    pub payment_id: [u8; 32],
    pub tx_hash: String,
}

#[event]
pub struct PaymentRefunded {
    pub payment_id: [u8; 32],
    pub refund_amount: u64,
}

#[event]
pub struct PaymentRequestCreated {
    pub request_id: [u8; 32],
    pub merchant: Pubkey,
    pub amount: u64,
    pub description: String,
}

#[event]
pub struct RequestPaymentReceived {
    pub request_id: [u8; 32],
    pub payer: Pubkey,
}

#[error_code]
pub enum PayChainError {
    #[msg("Payment is not in failed status")]
    PaymentNotFailed,
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Invalid message data length")]
    InvalidMessageData,
    #[msg("Payment request already paid")]
    AlreadyPaid,
    #[msg("Payment request expired")]
    RequestExpired,
}
