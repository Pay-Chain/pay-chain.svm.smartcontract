use anchor_lang::prelude::*;

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
