use anchor_lang::prelude::*;

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
