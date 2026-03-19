use anchor_lang::prelude::*;

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
pub enum PaymentStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Refunded,
}
