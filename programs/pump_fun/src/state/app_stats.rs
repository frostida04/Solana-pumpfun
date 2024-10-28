use anchor_lang::prelude::*;

#[account]
pub struct AppStats {
  pub owner: Pubkey,
  pub fee_lamports: u64,
  pub fee_account: Pubkey,
}