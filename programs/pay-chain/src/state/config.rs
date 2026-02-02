#[account]
#[derive(InitSpace)]
pub struct Config {
    pub authority: Pubkey,
    pub fee_recipient: Pubkey,
    pub router: Pubkey,
    pub fixed_base_fee: u64,
    pub fee_rate_bps: u16,
    #[max_len(64)]
    pub chain_id: String,
    pub bump: u8,
}
